use clap::Parser;
use git2::Repository;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use tera::{Context, Tera};

// ================================================
// ========== COMMAND LINE ARGUMENTS ==============
// ================================================

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The name of the project to scaffold.
    #[arg(short, long, default_value = "MyExampleProject")]
    project_name: String,

    /// The output directory where the generated files will be placed.
    #[arg(short, long, default_value = "generated")]
    output: String,

    /// The configuration file path.
    #[arg(short, long, default_value = "scaffolding.toml")]
    config: String,
}

// ================================================
// ========== UTILITY FUNCTIONS ===================
// ================================================

/// Check if the given repository URL is local.
/// We assume it is local if it doesn't start with "http://", "https://", or "git://".
fn is_local_repo(repo_url: &str) -> bool {
    !repo_url.starts_with("http://") && !repo_url.starts_with("https://") && !repo_url.starts_with("git://")
}

/// Obtain the repository at `repo_url`.
/// If it is a local repository, open it directly;
/// if remote, clone it to the specified destination (`dest`).
fn obtain_template_repo(repo_url: &str, dest: &Path) -> Result<Repository, Box<dyn Error>> {
    if is_local_repo(repo_url) {
        let repo = Repository::open(repo_url)?;
        Ok(repo)
    } else {
        let repo = Repository::clone(repo_url, dest)?;
        Ok(repo)
    }
}

/// Run a hook script located at `script_path`.
fn run_hook(script_path: &Path) -> io::Result<()> {
    let status = Command::new(script_path).status()?;
    if !status.success() {
        Err(io::Error::new(io::ErrorKind::Other, "Hook script failed"))
    } else {
        Ok(())
    }
}

// ================================================
// ========== DATA STRUCTURES =====================
// ================================================

#[derive(Deserialize)]
struct TemplateFile {
    src: String,
    dest: String,
}

#[derive(Deserialize)]
struct TemplateConfig {
    files: Vec<TemplateFile>,
}

#[derive(Deserialize)]
struct HooksConfig {
    pre: Option<String>,
    post: Option<String>,
}

/// Represents a single scaffold configuration. Each scaffold specifies:
/// - A name (optional)
/// - A repository URL (local or remote)
/// - An optional template directory (defaults to "templates" if not provided)
/// - A set of template file definitions
/// - Optional hooks to run before and after generation.
#[derive(Deserialize)]
struct Scaffold {
    name: Option<String>,
    repo: String,
    template_dir: Option<String>,
    template: TemplateConfig,
    hooks: Option<HooksConfig>,
    variables: Option<HashMap<String, toml::Value>>,
}

#[derive(Deserialize)]
struct Config {
    scaffolds: Vec<Scaffold>,
}

/// Load the configuration from a TOML file at `config_path`.
fn load_config(config_path: &Path) -> Result<Config, Box<dyn Error>> {
    let config_str = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_str)?;
    Ok(config)
}

// ================================================
// ========== TEMPLATE RENDERING ==================
// ================================================

/// Render the templates (or copy files) for a given scaffold.
///
/// For files whose `src` ends with ".tera", the file is processed
/// using Tera with the provided context; for other files, the file is
/// copied verbatim to the destination.
///
/// # Arguments
/// * `templates_dir` - The base directory containing the scaffold’s files.
/// * `output_base` - The directory where rendered/copied files will be written.
/// * `scaffold` - The scaffold configuration.
/// * `context_data` - The Tera context used for rendering templates.
fn render_templates(templates_dir: &Path, output_base: &Path, scaffold: &Scaffold, context_data: &Context) -> Result<(), Box<dyn Error>> {
    // Create a Tera instance for templating.
    let mut tera = Tera::default();

    // Add template files (those ending in ".tera") to the Tera instance.
    for file in &scaffold.template.files {
        if file.src.ends_with(".tera") {
            // Determine the key (template name) by stripping "templates/" if present.
            let key = if file.src.starts_with("templates/") {
                &file.src["templates/".len()..]
            } else {
                &file.src
            };

            let file_path = templates_dir.join(&file.src);
            tera.add_template_file(file_path, Some(key))?;
        }
    }

    // Process each file defined in the scaffold.
    for file in &scaffold.template.files {
        // Replace placeholder(s) in the destination path.
        let dest_path_str = file.dest.replace(
            "{{project_name}}",
            context_data.get("project_name").and_then(|v| v.as_str()).unwrap_or("default"),
        );
        let dest_path = output_base.join(dest_path_str);

        // Ensure the destination directory exists.
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if file.src.ends_with(".tera") {
            // For template files, render them using Tera.
            let key = if file.src.starts_with("templates/") {
                &file.src["templates/".len()..]
            } else {
                &file.src
            };
            let rendered = tera.render(key, context_data)?;
            fs::write(dest_path, rendered)?;
        } else {
            // For non-template files, copy them verbatim.
            let source_path = templates_dir.join(&file.src);
            fs::copy(source_path, dest_path)?;
        }
    }

    Ok(())
}

// ================================================
// ========== SCAFFOLD PROCESSING =================
// ================================================

/// Process a single scaffold: obtain its repository, render its templates,
/// and run any pre- or post-generation hooks.
fn process_scaffold(scaffold: &Scaffold, project_name: &str, output_base: &Path) -> Result<(), Box<dyn Error>> {
    // --- Obtain the Scaffold Repository ---
    let scaffold_repo_base: PathBuf = if is_local_repo(&scaffold.repo) {
        // For a local repository, canonicalize the path.
        let path = fs::canonicalize(&scaffold.repo)?;
        println!("Using local scaffold repository at: {:?}", path);
        path
    } else {
        // For a remote repository, clone it into a temporary directory.
        let temp_dir = TempDir::new()?;
        let scaffold_dir = temp_dir.path().join(scaffold.name.as_deref().unwrap_or("unnamed"));
        if scaffold_dir.exists() {
            fs::remove_dir_all(&scaffold_dir)?;
        }
        println!("Cloning repo {:?}", scaffold.repo);
        let _repo = obtain_template_repo(&scaffold.repo, &scaffold_dir)?;
        scaffold_dir
    };

    // --- Determine the Templates Directory ---
    // Use the provided template_dir or default to "templates".
    let templates_dir = scaffold_repo_base.join(scaffold.template_dir.as_deref().unwrap_or("templates"));
    println!("Rendering templates from: {:?}", templates_dir);

    // --- Set Up the Templating Context ---
    let mut context = Context::new();
    context.insert("project_name", project_name);

    // Merge additional variables from scaffolding.toml (if any)
    if let Some(vars) = &scaffold.variables {
        for (key, value) in vars {
            // Insert each variable into the Tera context.
            println!("Setting variable: {} = {:?}", key, value);
            context.insert(key, value);
        }
    }

    // --- Render Templates / Copy Files ---
    render_templates(&templates_dir, output_base, scaffold, &context)?;

    // --- Run Pre-Generation Hook (if any) ---
    if let Some(hooks) = &scaffold.hooks {
        if let Some(pre_script) = &hooks.pre {
            let pre_hook_path = scaffold_repo_base.join(pre_script);
            println!("Running pre-generation hook: {:?}", pre_hook_path);
            run_hook(&pre_hook_path)?;
        }
    }

    // --- Run Post-Generation Hook (if any) ---
    if let Some(hooks) = &scaffold.hooks {
        if let Some(post_script) = &hooks.post {
            let post_hook_path = scaffold_repo_base.join(post_script);
            println!("Running post-generation hook: {:?}", post_hook_path);
            run_hook(&post_hook_path)?;
        }
    }

    Ok(())
}

// ================================================
// ========== MAIN FUNCTION =======================
// ================================================

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let project_name = args.project_name;
    let output_base = Path::new(&args.output);
    let config_path = Path::new(&args.config);

    // --- Load the Client-Side Configuration ---
    println!("Loading configuration from: {:?}", config_path);
    let config = load_config(config_path)?;

    // --- Process Each Scaffold ---
    for scaffold in &config.scaffolds {
        if let Some(name) = &scaffold.name {
            println!("Processing scaffold: {}", name);
        } else {
            println!("Processing unnamed scaffold");
        }
        process_scaffold(scaffold, &project_name, output_base)?;
    }

    println!("Scaffolding for project '{}' created successfully!", project_name);
    Ok(())
}
