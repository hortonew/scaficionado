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
[![Build Status][docsrs-badge]][docsrs-url]

[crates-badge]: https://img.shields.io/crates/v/scaficionado.svg
[crates-url]: https://crates.io/crates/scaficionado
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/hortonew/scaficionado/blob/main/LICENSE
[actions-badge]: https://github.com/hortonew/scaficionado/actions/workflows/release.yml/badge.svg
[actions-url]: https://github.com/hortonew/scaficionado/actions
[docsrs-badge]: https://docs.rs/scaficionado/badge.svg
[docsrs-url]: https://docs.rs/scaficionado/latest/scaficionado/

## Prerequisites

- A scaffolding.toml file (can be renamed if using -c argument).  See Configuration Details below.

## Install

```sh
cargo install scaficionado
```

## Usage

```sh
scaficionado -h
# Usage: scaficionado [OPTIONS]

# Options:
#   -p, --project-name <PROJECT_NAME>  The name of the project to scaffold [default: MyExampleProject]
#   -o, --output <OUTPUT>              The output directory where the generated files will be placed [default: generated]
#   -c, --config <CONFIG>              The configuration file path [default: scaffolding.toml]
#   -h, --help                         Print help
#   -V, --version                      Print version

# accept defaults
scaficionado

# with flags for output and project name
scaficionado -o output_scaffolding -p MyTestProjectName
```

![Scaficionado](/images/scaficionado.gif)

## Configuration Details

```toml
[[scaffolds]] # Array:
# Each [[scaffolds]] entry defines a separate scaffold. The generator processes each scaffold in order.

# A friendly name for the scaffold (used for logging), which has no bearing on the configuration
name = 

# The repository from which to pull the scaffold files. This can be either a local path (e.g. ../example-1) or a remote Git URL.
repo =

# The directory within the repository that contains your templates. Defaults to "templates" if not provided. For repositories where the files reside at the root, set this to ".".
# (optional): defaults to "templates" if not provided.
template_dir = "."

# Under the template table, list the files to process. Each file entry has a src and dest, and dest paths can use the {{project_name}} variable.
[scaffolds.template]
files = [
    {src = "src1.ext.tera", dest = "dest1/src1.ext"},
    {src = "src2.ext", dest = "dest2/src2.ext"},
    {src = "src3.ext", dest = "{{project_name}}/dest3/src3.ext"},
]

# Optionally specify hook scripts to run before (pre) and after (post) template rendering. These paths are relative to the repository root.
[scaffolds.hooks]
# optional pre hook found in the remote repository
pre = "hooks/pre.sh"
# optional post hook found in the remote repository
post = "hooks/post.sh"

# Optionally specify key/values to get injected into the context
[scaffolds.variables]
some_count = 2
some_environment = "development"
```

## Expanded variables

- project_name: if used in scaffolding.toml, this path will get expanded (e.g. scaficionado -n TestProjectOne)

## Example configuration

See [example scaffolding](examples/scaffolding.toml) using this project.

## Build from source

```sh
cargo build --release
cargo install --path .
```

## Release

To release a new version to Crates.io, tag a new version as vX.Y.Z, matching the version in Cargo.toml.
