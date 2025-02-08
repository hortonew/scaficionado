use git2::Repository;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use tera::{Context, Tera};

// A helper function that checks if the repo_url looks like a local path.
fn is_local_repo(repo_url: &str) -> bool {
    // assume local if it doesn't start with http://, https://, or git://
    !repo_url.starts_with("http://") && !repo_url.starts_with("https://") && !repo_url.starts_with("git://")
}

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

#[derive(Deserialize)]
struct Scaffold {
    name: Option<String>,
    template: TemplateConfig,
    hooks: Option<HooksConfig>,
}
#[derive(Deserialize)]
struct Config {
    scaffolds: Vec<Scaffold>,
}

fn load_config(config_path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    let config_str = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_str)?;
    Ok(config)
}

fn run_hook(script_path: &Path) -> io::Result<()> {
    let status = Command::new(script_path).status()?;
    if !status.success() {
        Err(io::Error::new(io::ErrorKind::Other, "Hook script failed"))
    } else {
        Ok(())
    }
}

fn render_templates(templates_dir: &Path, output_base: &Path, scaffold: &Scaffold, context_data: &Context) -> Result<(), Box<dyn Error>> {
    // Build the Tera instance with all files under the templates directory.
    let templates_pattern = format!("{}/**/*", templates_dir.to_str().unwrap());
    let tera = Tera::new(&templates_pattern)?;

    for file in &scaffold.template.files {
        // If the file.src starts with "templates/", remove that prefix.
        let key = if file.src.starts_with("templates/") {
            &file.src["templates/".len()..]
        } else {
            &file.src
        };

        // Render the template using the adjusted key.
        let rendered = tera.render(key, context_data)?;

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

        // Write the rendered content to the file.
        fs::write(dest_path, rendered)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Define parameters.
    let repo_url = "../example-1";
    let project_name = "MyLab";
    let output_base = Path::new("./generated");

    // Determine the repository base directory.
    let repo_base: PathBuf = if is_local_repo(repo_url) {
        let path = fs::canonicalize(repo_url)?;
        println!("Using local repository at: {:?}", path);
        path
    } else {
        let temp_dir = std::env::temp_dir().join("scaffold_repo");
        println!("Cloning remote repository into: {:?}", temp_dir);
        let _repo = obtain_template_repo(repo_url, &temp_dir)?;
        temp_dir
    };

    // Load the configuration (now containing multiple scaffolds).
    let config_path = repo_base.join("scaffolding.toml");
    println!("Loading configuration from: {:?}", config_path);
    let config = load_config(&config_path)?;

    // Set up the templating context.
    let mut context = Context::new();
    context.insert("project_name", project_name);

    // Iterate over each scaffold.
    for scaffold in config.scaffolds {
        if let Some(name) = scaffold.name.as_deref() {
            println!("Processing scaffold: {}", name);
        } else {
            println!("Processing unnamed scaffold");
        }

        // Process pre-generation hook, if defined.
        if let Some(hooks) = &scaffold.hooks {
            if let Some(pre_script) = &hooks.pre {
                let pre_hook_path = repo_base.join(pre_script);
                println!("Running pre-generation hook: {:?}", pre_hook_path);
                run_hook(&pre_hook_path)?;
            }
        }

        // Render the templates for this scaffold.
        let templates_dir = repo_base.join("templates");
        println!("Rendering templates from: {:?}", templates_dir);
        render_templates(&templates_dir, output_base, &scaffold, &context)?;

        // Process post-generation hook, if defined.
        if let Some(hooks) = &scaffold.hooks {
            if let Some(post_script) = &hooks.post {
                let post_hook_path = repo_base.join(post_script);
                println!("Running post-generation hook: {:?}", post_hook_path);
                run_hook(&post_hook_path)?;
            }
        }
    }

    println!("Scaffolding for project '{}' created successfully!", project_name);
    Ok(())
}
