# Contributing to envoyctl

Thank you for your interest in contributing to envoyctl! This document provides guidelines and rules for contributing.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Coding Standards](#coding-standards)
- [Commit Guidelines](#commit-guidelines)
- [Pull Request Process](#pull-request-process)
- [Release Process](#release-process)
- [Issue Reporting](#issue-reporting)

---

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow
- Keep discussions on-topic

---

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/boniface/envoyctl.git
   cd envoyctl
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/boniface/envoyctl.git
   ```

---

## Development Setup

### Prerequisites

- **Rust** (stable, 1.70+)
- **Cargo** (comes with Rust)
- **Docker** (optional, for validation testing)
- **Envoy** (optional, for native validation)

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run clippy (linter)
cargo clippy -- -D warnings

# Format code
cargo fmt
```

### Running Locally

```bash
# Initialize a test workspace
cargo run -- init --dir ./test-workspace

# Validate configuration
cargo run -- validate --config-dir ./test-workspace/config --out-dir ./test-workspace/out

# Build configuration only
cargo run -- build --config-dir ./test-workspace/config --out-dir ./test-workspace/out
```

---

## Coding Standards

### Rust Style

- **Follow Rust idioms**: Use `Result` for error handling, prefer iterators over loops where appropriate
- **Use `rustfmt`**: Run `cargo fmt` before committing
- **Pass `clippy`**: Run `cargo clippy -- -D warnings` with no warnings
- **Document public APIs**: All public functions, structs, and modules should have doc comments

### Code Organization

```
src/
├── main.rs        # Entry point, CLI dispatch
├── cli.rs         # CLI argument definitions (clap)
├── model.rs       # Data structures for config fragments
├── load.rs        # YAML loading and parsing
├── validate.rs    # Semantic validation of loaded config
├── generate.rs    # Envoy YAML generation
├── apply.rs       # Build, validate, apply commands
├── exec.rs        # External command execution (docker, envoy)
└── init.rs        # Workspace initialization
```

### Error Handling

- Use `anyhow::Result` for application errors
- Use `thiserror` for defining custom error types if needed
- Provide context with `.context()` or `.with_context()`
- Never use `.unwrap()` in library code; use `.expect()` only with clear messages

### Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Functions | `snake_case` | `load_domains()` |
| Structs | `PascalCase` | `DomainSpec` |
| Constants | `SCREAMING_SNAKE_CASE` | `DEFAULT_TIMEOUT` |
| Modules | `snake_case` | `mod validate;` |
| Files | `snake_case.rs` | `validate.rs` |

### Dependencies

- Minimize dependencies; prefer stdlib when possible
- Check for security advisories before adding new crates
- Pin major versions in `Cargo.toml`
- Document why each dependency is needed

---

## Commit Guidelines

### Commit Message Format

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short description>

[optional body]

[optional footer]
```

### Types

| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `style` | Formatting, no code change |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `perf` | Performance improvement |
| `test` | Adding or updating tests |
| `chore` | Maintenance tasks |
| `ci` | CI/CD changes |

### Examples

```
feat(generate): add support for custom access log formats

fix(validate): handle missing upstream references gracefully

docs(readme): update installation instructions for Debian

refactor(load): extract common YAML parsing logic

chore(deps): update clap to 4.5
```

### Rules

- Keep the first line under 72 characters
- Use imperative mood ("add" not "added")
- Reference issues when applicable: `Fixes #123`
- One logical change per commit

---

## Pull Request Process

### Before Submitting

1. **Sync with upstream**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run all checks**:
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   cargo build --release
   ```

3. **Update documentation** if needed

### PR Requirements

- [ ] Clear title following commit conventions
- [ ] Description explaining what and why
- [ ] All CI checks pass
- [ ] Tests added for new functionality
- [ ] Documentation updated if applicable
- [ ] No merge conflicts

### Review Process

1. Submit PR against `main` branch
2. Wait for CI checks to pass
3. Address reviewer feedback
4. Maintainer approves and merges

### Branch Naming

```
feat/<short-description>
fix/<short-description>
docs/<short-description>
refactor/<short-description>
```

---

## Release Process

### Versioning

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes to CLI or config format
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

### Release Checklist

1. **Update version** in `Cargo.toml`
2. **Update changelog** in `debian/changelog`
3. **Create release commit**:
   ```bash
   git commit -am "chore(release): v1.2.3"
   ```
4. **Tag the release**:
   ```bash
   git tag -a v1.2.3 -m "Release v1.2.3"
   git push origin main --tags
   ```
5. **CI builds and publishes** the Debian package

### Changelog Format

```
envoyctl (1.2.3) stable; urgency=medium

  * feat: Add new feature X
  * fix: Resolve issue with Y
  * docs: Update README

 -- Your Name <email@example.com>  Wed, 25 Dec 2025 12:00:00 +0000
```

---

## Issue Reporting

### Bug Reports

Include:

- **envoyctl version**: `envoyctl --version`
- **OS and architecture**: e.g., Debian 12, amd64
- **Steps to reproduce**
- **Expected behavior**
- **Actual behavior**
- **Relevant config snippets** (sanitized)
- **Error messages** (full output)

### Feature Requests

Include:

- **Use case**: Why do you need this?
- **Proposed solution**: What should it look like?
- **Alternatives considered**: What else could work?

### Labels

| Label | Description |
|-------|-------------|
| `bug` | Something isn't working |
| `enhancement` | New feature request |
| `documentation` | Documentation improvement |
| `good first issue` | Good for newcomers |
| `help wanted` | Extra attention needed |

---

## Questions?

- Open a GitHub Discussion for general questions
- Check existing issues before creating new ones
- Be patient; maintainers are volunteers

---

Thank you for contributing to envoyctl!!

