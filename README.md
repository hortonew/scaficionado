# Scaficionado

A scaffolding tool to create repeatable project structure using files and scripts from local or remote repositories.

- Files ending in .tera will get templated using [tera](https://crates.io/crates/tera), otherwise they'll just get copied.

- Files will get generated into a directory called "generated" in the root of where it's called.

## Prerequisites

- A scaffolding.toml file (can be renamed if using -c argument).  See Configuration Details below.

## Usage

```sh
# create scaffolding.toml

cargo install scaficionado

scaficionado -h
# Usage: scaficionado [OPTIONS]

# Options:
#   -p, --project-name <PROJECT_NAME>  The name of the project to scaffold [default: MyExampleProject]
#   -o, --output <OUTPUT>              The output directory where the generated files will be placed [default: generated]
#   -c, --config <CONFIG>              The configuration file path (default: scaffolding.toml) [default: scaffolding.toml]
#   -h, --help                         Print help
#   -V, --version                      Print version

scaficionado -o output_scaffolding -p MyTestProjectName
```

## Configuration Details

```toml
[[scaffolds]] # Array:
# Each [[scaffolds]] entry defines a separate scaffold. The generator processes each scaffold in order.

# A friendly name for the scaffold (used for logging).
name = # The name of your config which has no bearing on the configuration

# The repository from which to pull the scaffold files. This can be either a local path (e.g. ../example-1) or a remote Git URL.
repo =

# The directory within the repository that contains your templates. Defaults to "templates" if not provided. For repositories where the files reside at the root, set this to ".".
template_dir = # (optional): defaults to "templates" if not provided.

# Under the template table, list the files to process. Each file entry has:
[scaffolds.template]
src = # The source file path relative to the template directory.
dest = # The destination path where the file should be generated. Use {{project_name}} as a placeholder.

# Optionally specify hook scripts to run before (pre) and after (post) template rendering. These paths are relative to the repository root.
[scaffolds.hooks]
pre = # optional pre hook e.g. hooks/pre.sh
post = # optional post hook e.g. hooks/post.sh
```

## Expanded variables

- project_name: if used in scaffolding.toml, this path will get expanded (e.g. scaficionado -n TestProjectOne)

## Example configuration

```toml
# Create array scaffolds to define multiple scaffolds.  Define multiple templates and hooks for each scaffold.
[[scaffolds]]
name = "base"
repo = "../example-1"      # This can be a local path or a remote URL.
template_dir = "templates" # Optional; defaults to "templates" if not provided.
[scaffolds.template]
files = [
    { src = "kind_config.yaml.tera", dest = "example-1/{{project_name}}/kind_config.yaml" },
    { src = "terraform/main.tf", dest = "example-1/{{project_name}}/main.tf" },
    { src = "terraform/variables.tf", dest = "example-1/{{project_name}}/variables.tf" },
    { src = "terraform/terraform.tf", dest = "example-1/{{project_name}}/terraform.tf" },
]
[scaffolds.hooks]
pre = "hooks/pre_generate.sh"
post = "hooks/post_generate.sh"

# Another local repo to scaffold
[[scaffolds]]
name = "extras"
repo = "../example-1"
[scaffolds.template]
files = [
    { src = "extra_config.yaml.tera", dest = "example-1/{{project_name}}/extra_config.yaml" },
]
[scaffolds.hooks]
pre = "hooks/extra_pre.sh"
post = "hooks/extra_post.sh"

# A remote repository used to scaffold the project.
[[scaffolds]]
name = "RemoteLab"
repo = "https://github.com/hortonew/monitoring-lab.git"
template_dir = "."
[scaffolds.template]
files = [
    { src = "docker-compose.yml", dest = "example-1/{{project_name}}/docker-compose.yml" },
]

# A remote repository used to scaffold a new project
[[scaffolds]]
name = "RemoteLab2"
repo = "https://github.com/hortonew/monitoring-lab-k8s.git"
template_dir = "."
[scaffolds.template]
files = [
    { src = "configure-k8s-cluster.yml", dest = "example-1-isolated/configure-k8s-cluster.yml" },
]
```

## Build from source

```sh
cargo build --release
cargo install --path .
```