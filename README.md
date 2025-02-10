# Scaficionado

Pronunciation: sk[a·fi·ci·o·na·do](https://www.oxfordlearnersdictionaries.com/us/definition/english/aficionado), or scaffold + aficionado.

A scaffolding tool to create repeatable project structure using files and scripts from local or remote repositories.

- Files ending in .tera will get templated using [tera](https://keats.github.io/tera/), otherwise they'll just get copied.
- Files will get generated into a directory called "generated" in the root of where it's called.
- Hooks (shell scripts) can be called before and after each scaffold

## Status

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Docs][docsrs-badge]][docsrs-url]
[![Downloads][downloads-badge]][downloads-url]


[crates-badge]: https://img.shields.io/crates/v/scaficionado.svg
[crates-url]: https://crates.io/crates/scaficionado
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/hortonew/scaficionado/blob/main/LICENSE
[actions-badge]: https://github.com/hortonew/scaficionado/actions/workflows/release.yml/badge.svg
[actions-url]: https://github.com/hortonew/scaficionado/actions
[docsrs-badge]: https://docs.rs/scaficionado/badge.svg
[docsrs-url]: https://docs.rs/scaficionado/latest/scaficionado/
[downloads-badge]: https://img.shields.io/crates/d/scaficionado.svg
[downloads-url]: https://crates.io/crates/scaficionado

## Prerequisites

- A scaffolding.toml file (can be renamed if using -c argument).  See Configuration Details below.
- Cargo (`curl https://sh.rustup.rs -sSf | sh`)

## Install

```sh
cargo install scaficionado
```

## Usage

```sh
scaficionado -h
# Usage: scaficionado [OPTIONS]

# Options:
#   -p, --project-name <PROJECT_NAME>  The name of the project to scaffold.  Overwrites project_name set in configuration file [default: MyExampleProject]
#   -o, --output <OUTPUT>              The output directory where the generated files will be placed.  Overwrites output set in configuration file [default: generated]
#   -c, --config <CONFIG>              The configuration file path [default: scaffolding.toml]
#   -w, --overwrite                    Overwrite existing files if set. [default: false].  Overwrites overwrite=false set in configuration file
#   -h, --help                         Print help
#   -V, --version                      Print version

# accept defaults
scaficionado

# with flags for project name and output
scaficionado -p MyTestProjectName -o output_scaffolding

# force overwrite existing files in current working directory
scaficionado -p MyTestProjectName -o . -w
```

![Scaficionado](/images/scaficionado.gif)

## Pro Tip

It's hard to spell, so use an alias!

```sh
echo "ska='scaficionado'" >> ~/.zshrc
source ~/.zshrc
ska --version
```


## Configuration Details

```toml
# Project section (optional)
[project]
# Project name (overwrites default). Overwritten by --project-name argument.
name = "MyExampleProject"
# Output directory (overwrites default). Overwritten by --output argument.
# Warning: using "." will overwrite files in the current directory.
output = "generated"
# Controls whether existing files are overwritten (default: false).
overwrite = false

# Scaffolds array
[[scaffolds]]
# Friendly name for the scaffold (used for logging).
name = "Example Scaffold"
# Repository for scaffold files (local path or remote Git URL).
repo = "../example-1"
# Directory within the repository containing templates (default: "templates").
template_dir = "."

# Template files to process
[scaffolds.template]
# List of files that map source repository files to templated destination files in the output location.
# Destination names can use variables defined below (e.g. {{some_environment}}-{{some_count}}).
# {{project_name}} is a reserved variable that comes from project.name (see above).
files = [
    {src = "src1.ext.tera", dest = "dest1/src1.ext"},
    {src = "src2.ext", dest = "dest2/src2.ext"},
    {src = "src3.ext", dest = "{{project_name}}-{{some_environment}}-{{some_count}}/dest3/src3.ext"},
]

# Hook scripts (optional)
[scaffolds.hooks]
pre = "hooks/pre.sh"  # Pre-render hook script
post = "hooks/post.sh"  # Post-render hook script

# Variables to inject into the context (optional)
[scaffolds.variables]
some_count = 2
some_environment = "development"
```

## Advanced configuration

You can render an entire directory (recursively) if you want.  For example:

```toml
[project]
name = "TemplateRepoDirectory"
output = "."

[[scaffolds]]
name = "Local Repo"
repo = "/some/local/repo"
template_dir = "."

[scaffolds.template]
files = [
    { src = "templates", dest = "templates" },
]

[scaffolds.variables]
kind_workers = 3
environment = "development"
```

It will put the "templates" directory in your current working directory, and any file with a .tera extension will:

1. get templated
2. have the .tera extension removed

## Expanded variables

Expanded variables apply to the dest section of the scaffold, as well as the expanded tera templated file.

- project_name: if used in scaffolding.toml, this path will get expanded (e.g. scaficionado -n TestProjectOne)
- key/values defined under scaffolds.variables (e.g. {{some_count}}) would expand to 2 in the above example

## Example configuration

- [example scaffolding for Kubernetes](examples/scaffolding.toml)
- [example scaffolding for Rust AI app](examples/scaffolding-rust-ai.toml)

## Build from source

```sh
cargo build --release
cargo install --path .
```

## Release

To release a new version to Crates.io, tag a new version as vX.Y.Z, matching the version in Cargo.toml.

## Some ways to use this

Store scaffolding files in:

~/scaffolding/rust-github.toml

- so you can `ska -c ~/scaffolding/rust-github -o .` to generate a useful CI/CD workflow and Makefile.

~/scaffolding/eks-cheap.toml

- so you can `ska -c ~/scaffolding/eks-cheap.toml -o .` to generate a quick EKS config for a new lab environment.
