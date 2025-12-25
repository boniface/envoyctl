# Source code

This directory contains the Rust source code for `envoyctl`.

## Structure

- cli.rs        Command-line interface
- init.rs       Workspace initialization
- load.rs       Load and parse configuration fragments
- validate.rs   Cross-fragment validation
- generate.rs   Envoy configuration generation
- apply.rs      Install and restart logic

The code is intentionally structured to mirror the configuration model.
