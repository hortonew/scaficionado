use git2::Repository;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use tera::{Context, Tera};

/// A helper function that checks if the repo URL looks like a local path.
fn is_local_repo(repo_url: &str) -> bool {
    // Assume local if it doesn't start with "http://", "https://", or "git://"
    !repo_url.starts_with("http://") && !repo_url.starts_with("https://") && !repo_url.starts_with("git://")
}

/// Given a repository URL and a destination, obtain the template repository.
/// If the repository is local, simply open it; if remote, clone it.
fn obtain_template_repo(repo_url: &str, dest: &Path) -> Result<Repository, Box<dyn std::error::Error>> {
    if is_local_repo(repo_url) {
        let repo = Repository::open(repo_url)?;
        Ok(repo)
    } else {
        let repo = Repository::clone(repo_url, dest)?;
        Ok(repo)
    }
}

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

/// Each scaffold entry now includes its own repo URL and optional template directory.
#[derive(Deserialize)]
struct Scaffold {
    name: Option<String>,
    repo: String,
    template_dir: Option<String>, // Defaults to "templates" if not provided.
    template: TemplateConfig,
    hooks: Option<HooksConfig>,
}

#[derive(Deserialize)]
struct Config {
    scaffolds: Vec<Scaffold>,
}

/// Load the configuration from a local file.
fn load_config(config_path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    let config_str = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_str)?;
    Ok(config)
}

/// Run a hook script given its path.
fn run_hook(script_path: &Path) -> io::Result<()> {
    let status = Command::new(script_path).status()?;
    if !status.success() {
        Err(io::Error::new(io::ErrorKind::Other, "Hook script failed"))
    } else {
        Ok(())
    }
}

/// Render templates for a given scaffold. For files whose src ends with ".tera",
/// they will be processed as templates using Tera. For all other files, they are copied
/// verbatim from the source.
fn render_templates(templates_dir: &Path, output_base: &Path, scaffold: &Scaffold, context_data: &Context) -> Result<(), Box<dyn Error>> {
    // Create an empty Tera instance for templating files.
    let mut tera = Tera::default();

    // First, add all files that end with ".tera" to the Tera instance.
    for file in &scaffold.template.files {
        if file.src.ends_with(".tera") {
            // Determine the key under which this template will be registered.
            // (Optionally, strip a leading "templates/" if present.)
            let key = if file.src.starts_with("templates/") {
                &file.src["templates/".len()..]
            } else {
                &file.src
            };

            // Construct the full path to the file.
            let file_path = templates_dir.join(&file.src);
            tera.add_template_file(file_path, Some(key))?;
        }
    }

    // Now process each file.
    for file in &scaffold.template.files {
        // Replace placeholders in the destination path.
        let dest_path_str = file.dest.replace(
            "{{project_name}}",
            context_data.get("project_name").and_then(|v| v.as_str()).unwrap_or("default"),
        );
        let dest_path = output_base.join(dest_path_str);

        // Ensure the output directory exists.
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if file.src.ends_with(".tera") {
            // For template files, render using Tera.
            let key = if file.src.starts_with("templates/") {
                &file.src["templates/".len()..]
            } else {
                &file.src
            };
            let rendered = tera.render(key, context_data)?;
            fs::write(dest_path, rendered)?;
        } else {
            // For non-template files, simply copy them verbatim.
            let source_path = templates_dir.join(&file.src);
            fs::copy(source_path, dest_path)?;
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Define parameters.
    let project_name = "MyLab";
    let output_base = Path::new("./generated");

    // Load the client-side configuration (scaffolding.toml).
    let config_path = Path::new("scaffolding.toml");
    println!("Loading configuration from: {:?}", config_path);
    let config = load_config(config_path)?;

    // Set up the templating context.
    let mut context = Context::new();
    context.insert("project_name", project_name);

    // Iterate over each scaffold defined in the configuration.
    for scaffold in config.scaffolds {
        if let Some(name) = scaffold.name.as_deref() {
            println!("Processing scaffold: {}", name);
        } else {
            println!("Processing unnamed scaffold");
        }

        // For each scaffold, obtain the repository (local or remote) from which to pull files.
        let scaffold_repo_base: PathBuf = if is_local_repo(&scaffold.repo) {
            let path = fs::canonicalize(&scaffold.repo)?;
            println!("Using local scaffold repository at: {:?}", path);
            path
        } else {
            // Clone remote repository into a temporary directory (one per scaffold).
            let temp_dir = std::env::temp_dir().join("scaffold_repo");
            let scaffold_dir = temp_dir.join(scaffold.name.as_deref().unwrap_or("unnamed"));
            if scaffold_dir.exists() {
                fs::remove_dir_all(&scaffold_dir)?;
            }
            println!("Cloning remote scaffold repository into: {:?}", scaffold_dir);
            let _repo = obtain_template_repo(&scaffold.repo, &scaffold_dir)?;
            scaffold_dir
        };

        // Determine the templates directory for this scaffold.
        let templates_dir = scaffold_repo_base.join(scaffold.template_dir.as_deref().unwrap_or("templates"));
        println!("Rendering templates from: {:?}", templates_dir);
        render_templates(&templates_dir, output_base, &scaffold, &context)?;

        // Process pre- and post-generation hooks if defined.
        if let Some(hooks) = &scaffold.hooks {
            if let Some(pre_script) = &hooks.pre {
                let pre_hook_path = scaffold_repo_base.join(pre_script);
                println!("Running pre-generation hook: {:?}", pre_hook_path);
                run_hook(&pre_hook_path)?;
            }
            if let Some(post_script) = &hooks.post {
                let post_hook_path = scaffold_repo_base.join(post_script);
                println!("Running post-generation hook: {:?}", post_hook_path);
                run_hook(&post_hook_path)?;
            }
        }
    }

    println!("Scaffolding for project '{}' created successfully!", project_name);
    Ok(())
}
