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
        // Replace placeholders in the destination path using Tera context.
        let dest_path_str = Tera::one_off(&file.dest, context_data, false)?;
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
///
/// Returns an Option<PathBuf> containing the persistent temporary directory
/// used for a remote clone, if applicable.
fn process_scaffold(scaffold: &Scaffold, project_name: &str, output_base: &Path) -> Result<Option<PathBuf>, Box<dyn Error>> {
    // --- Obtain the Scaffold Repository ---
    let scaffold_repo_base: PathBuf = if is_local_repo(&scaffold.repo) {
        // For a local repository, canonicalize the path.
        let path = fs::canonicalize(&scaffold.repo)?;
        println!("Using local scaffold repository at: {:?}", path);
        path
    } else {
        // For a remote repository, clone it into a temporary directory and persist it.
        let temp_dir = TempDir::new()?;
        let scaffold_dir = temp_dir.path().join(scaffold.name.as_deref().unwrap_or("unnamed"));
        if scaffold_dir.exists() {
            fs::remove_dir_all(&scaffold_dir)?;
        }
        println!("Cloning repo {:?}", scaffold.repo);
        let _repo = obtain_template_repo(&scaffold.repo, &scaffold_dir)?;
        // Persist the temporary directory so that the clone isn't deleted.
        let persistent_temp_dir = temp_dir.into_path();
        // The actual clone path is inside the persistent_temp_dir.
        persistent_temp_dir.join(scaffold.name.as_deref().unwrap_or("unnamed"))
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

    // If the repository was remote, return the persistent temporary directory so it can be cleaned up.
    if is_local_repo(&scaffold.repo) {
        Ok(None)
    } else {
        // The persistent directory is the same as scaffold_repo_base in the remote branch.
        Ok(Some(scaffold_repo_base))
    }
}

// ================================================
// ========== MAIN FUNCTION =======================
// ================================================

pub fn run() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let project_name = args.project_name;
    let output_base = Path::new(&args.output);
    let config_path = Path::new(&args.config);

    // --- Load the Client-Side Configuration ---
    println!("Loading configuration from: {:?}", config_path);
    let config = load_config(config_path)?;

    // Vector to hold persistent temporary directories (from remote clones)
    let mut persistent_dirs: Vec<PathBuf> = Vec::new();

    // --- Process Each Scaffold ---
    for scaffold in &config.scaffolds {
        if let Some(name) = &scaffold.name {
            println!("Processing scaffold: {}", name);
        } else {
            println!("Processing unnamed scaffold");
        }
        let maybe_dir = process_scaffold(scaffold, &project_name, output_base)?;
        if let Some(dir) = maybe_dir {
            // Save the persistent directory for cleanup later.
            persistent_dirs.push(dir);
        }
    }

    println!("Scaffolding for project '{}' created successfully!", project_name);

    // --- Clean Up Persistent Temporary Directories ---
    for dir in persistent_dirs {
        println!("Cleaning up temporary clone at: {:?}", dir);
        fs::remove_dir_all(&dir)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;
    use tera::Context;

    // Test the is_local_repo helper function.
    #[test]
    fn test_is_local_repo() {
        // Local path should return true.
        assert!(is_local_repo("some/local/path"));
        // URLs should return false.
        assert!(!is_local_repo("https://github.com/example/repo.git"));
        assert!(!is_local_repo("http://example.com/repo"));
        assert!(!is_local_repo("git://example.com/repo"));
    }

    // Test loading configuration from a TOML string.
    #[test]
    fn test_load_config() -> Result<(), Box<dyn std::error::Error>> {
        let toml_content = r#"
[[scaffolds]]
name = "Local"
repo = "local_repo"
template_dir = "templates"

[scaffolds.template]
files = [
    { src = "kind_config.yaml.tera", dest = "kind_config.yaml" }
]

[scaffolds.hooks]
pre = "scripts/pre.sh"
post = "scripts/post.sh"

[scaffolds.variables]
kind_workers = 3
environment = "development"
"#;
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("scaffolding.toml");
        fs::write(&config_path, toml_content)?;

        let config = load_config(&config_path)?;
        assert_eq!(config.scaffolds.len(), 1);
        let scaffold = &config.scaffolds[0];
        assert_eq!(scaffold.name.as_deref(), Some("Local"));
        assert_eq!(scaffold.repo, "local_repo");
        Ok(())
    }

    // Test the render_templates function.
    #[test]
    fn test_render_templates() -> Result<(), Box<dyn std::error::Error>> {
        // Create temporary directories for templates and output.
        let templates_dir = TempDir::new()?;
        let output_dir = TempDir::new()?;

        // Write a simple Tera template file with a placeholder.
        let template_content = "Hello, {{ project_name }}!";
        let template_file_path = templates_dir.path().join("greeting.txt.tera");
        fs::write(&template_file_path, template_content)?;

        // Build a Scaffold instance that uses this template.
        let scaffold = Scaffold {
            name: Some("Test".to_string()),
            repo: "local_repo".to_string(),     // Not used in this test.
            template_dir: Some("".to_string()), // Use the root of the templates_dir.
            template: TemplateConfig {
                files: vec![TemplateFile {
                    src: "greeting.txt.tera".to_string(),
                    dest: "greeting.txt".to_string(),
                }],
            },
            hooks: None,
            variables: None,
        };

        // Create a Tera context and insert a value for project_name.
        let mut context = Context::new();
        context.insert("project_name", "TestProject");

        // Render the template.
        render_templates(templates_dir.path(), output_dir.path(), &scaffold, &context)?;

        // Verify the rendered output.
        let output_file_path = output_dir.path().join("greeting.txt");
        let rendered_content = fs::read_to_string(output_file_path)?;
        assert_eq!(rendered_content, "Hello, TestProject!");

        Ok(())
    }

    // Test process_scaffold with a simulated local repository.
    #[test]
    fn test_process_scaffold_local() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory to simulate a local repository.
        let local_repo_dir = TempDir::new()?;
        // Create the "templates" directory within it.
        let templates_subdir = local_repo_dir.path().join("templates");
        fs::create_dir_all(&templates_subdir)?;

        // Write a sample template into the local repo.
        let template_content = "Hello, {{ project_name }}!";
        let template_file_path = templates_subdir.join("greeting.txt.tera");
        fs::write(&template_file_path, template_content)?;

        // Build a Scaffold instance referring to the local repository.
        let scaffold = Scaffold {
            name: Some("LocalTest".to_string()),
            repo: local_repo_dir.path().to_string_lossy().to_string(),
            template_dir: Some("templates".to_string()),
            template: TemplateConfig {
                files: vec![TemplateFile {
                    src: "greeting.txt.tera".to_string(),
                    dest: "greeting.txt".to_string(),
                }],
            },
            hooks: None,
            variables: None,
        };

        // Create a temporary output directory.
        let output_dir = TempDir::new()?;

        // Process the scaffold.
        let result = process_scaffold(&scaffold, "LocalProject", output_dir.path())?;
        // For local repositories, process_scaffold should return Ok(None).
        assert!(result.is_none());

        // Verify that the rendered file has been created.
        let output_file_path = output_dir.path().join("greeting.txt");
        let rendered_content = fs::read_to_string(output_file_path)?;
        assert_eq!(rendered_content, "Hello, LocalProject!");

        Ok(())
    }

    // Test the run_hook function with a hook that succeeds.
    #[test]
    fn test_run_hook_success() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let script_path = temp_dir.path().join("hook.sh");

        // Create a simple script that exits successfully.
        #[cfg(unix)]
        fs::write(&script_path, "#!/bin/sh\nexit 0")?;
        #[cfg(windows)]
        fs::write(&script_path, "@echo off\r\nexit 0")?;

        // On Unix, ensure the script is executable.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms)?;
        }

        // run_hook should complete without error.
        run_hook(&script_path)?;
        Ok(())
    }

    // Test the run_hook function with a hook that fails.
    #[test]
    fn test_run_hook_failure() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let script_path = temp_dir.path().join("hook_fail.sh");

        // Create a simple script that exits with a non-zero status.
        #[cfg(unix)]
        fs::write(&script_path, "#!/bin/sh\nexit 1")?;
        #[cfg(windows)]
        fs::write(&script_path, "@echo off\r\nexit 1")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms)?;
        }

        // run_hook should return an error.
        let result = run_hook(&script_path);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_dest_file_expands_variables_from_scaffold_variables() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory to simulate a local repository.
        let local_repo_dir = TempDir::new()?;
        let templates_dir = local_repo_dir.path().join("templates");
        fs::create_dir_all(&templates_dir)?;

        // Create necessary sub-directory and template file.
        let kind_cluster_dir = templates_dir.join("kind-cluster");
        fs::create_dir_all(&kind_cluster_dir)?;
        let template_file_path = kind_cluster_dir.join("kind_config.yaml.tera");
        fs::write(
            &template_file_path,
            "config: {{project_name}}, workers: {{kind_workers}}, env: {{environment}}",
        )?;

        // Build a Scaffold instance referring to the local repository.
        let scaffold = Scaffold {
            name: Some("LocalTest".to_string()),
            repo: local_repo_dir.path().to_string_lossy().to_string(),
            template_dir: Some("templates".to_string()),
            template: TemplateConfig {
                files: vec![TemplateFile {
                    src: "kind-cluster/kind_config.yaml.tera".to_string(),
                    dest: "{{project_name}}-{{environment}}-kind_config{{kind_workers}}.yaml".to_string(),
                }],
            },
            hooks: None,
            variables: Some({
                let mut map = HashMap::new();
                map.insert("kind_workers".to_string(), toml::Value::Integer(3));
                map.insert("environment".to_string(), toml::Value::String("development".to_string()));
                map
            }),
        };

        // Create a temporary output directory.
        let output_dir = TempDir::new()?;
        process_scaffold(&scaffold, "MyProject", output_dir.path())?;

        // Verify that the destination filename has expanded variables.
        let expected_output_file = output_dir.path().join("MyProject-development-kind_config3.yaml");
        assert!(expected_output_file.exists());
        assert!(expected_output_file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("MyProject-development-kind_config3.yaml"));
        let output_content = fs::read_to_string(expected_output_file)?;
        assert!(output_content.contains("config: MyProject, workers: 3, env: development"));
        Ok(())
    }
}
