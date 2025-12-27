# Architecture Overview

This document describes the architecture of envoyctl and how its components interact.

---

## High-Level Architecture

```mermaid
graph TB
    subgraph "envoyctl CLI"
        A[init]
        B[build]
        C[validate]
    end

    subgraph "Core Pipeline"
        LOAD[load]
        VAL[validate]
        GEN[generate]
    end

    subgraph "Output"
        TEMPLATE[Template Copy]
        CONFIG[Envoy Config YAML]
    end

    A --> TEMPLATE
    B --> LOAD
    C --> LOAD
    D --> LOAD

    LOAD --> VAL
    VAL --> GEN
    GEN --> CONFIG
    CONFIG --> EXEC
```

---

## Component Diagram

```mermaid
graph TB
    subgraph CLI["CLI Layer (cli.rs)"]
        INIT[init]
        BUILD[build]
        VALIDATE[validate]
        APPLY[apply]
    end

    subgraph Core["Core Pipeline"]
        LOAD[load.rs<br/>Parse YAML fragments]
        VAL[validate.rs<br/>Semantic validation]
        GEN[generate.rs<br/>Generate Envoy config]
    end

    subgraph Execution["Execution Layer"]
        EXEC[exec.rs<br/>Run external commands]
        DOCKER[Docker]
        NATIVE[Native Envoy]
    end

    subgraph Storage["Storage"]
        FRAGMENTS[(Config Fragments)]
        OUTPUT[(Generated YAML)]
        INSTALL[(Installed Config)]
    end

    INIT --> TEMPLATES[(Templates)]
    
    BUILD --> LOAD
    VALIDATE --> LOAD
    APPLY --> LOAD
    
    LOAD --> FRAGMENTS
    LOAD --> VAL
    VAL --> GEN
    GEN --> OUTPUT
    
    VALIDATE --> EXEC
    APPLY --> EXEC
    
    EXEC --> DOCKER
    EXEC --> NATIVE
    
    APPLY --> INSTALL
```

---

## Module Responsibilities

### `main.rs`
Entry point. Parses CLI arguments and dispatches to appropriate command handlers.

### `cli.rs`
Defines the CLI structure using `clap`:
- Global options (config-dir, out-dir, install-path)
- Subcommands (init, build, validate, apply)

### `model.rs`
Data structures representing configuration fragments:
- `DomainSpec` - Domain routing configuration
- `UpstreamSpec` - Backend cluster definitions
- `PolicySpec` - Headers, rate limits, retries, timeouts
- `DefaultsSpec` - Global defaults
- `RuntimeSpec` - Validation and restart configuration

### `load.rs`
YAML loading and parsing:
- Reads all fragment files from directories
- Deserializes into model structs
- Handles file discovery and error reporting

### `validate.rs`
Semantic validation:
- Checks upstream references exist
- Validates policy references
- Ensures required fields are present
- Cross-references between fragments

### `generate.rs`
Envoy configuration generation:
- Transforms fragments into Envoy v3 config
- Builds listeners, clusters, routes
- Handles TLS configuration
- Produces final YAML output

### `apply.rs`
Command implementations:
- `cmd_build()` - Load, validate, generate
- `cmd_validate()` - Build + Envoy validation
- `cmd_apply()` - Validate + install + restart

### `exec.rs`
External command execution:
- Docker-based Envoy validation
- Native Envoy validation
- Docker Compose restarts
- systemd restarts

### `init.rs`
Workspace initialization:
- Copies template files to target directory
- Sets up directory structure

---

## Execution Modes

### Docker Validation (Recommended)

```mermaid
graph LR
    A[envoyctl] -->|mount config| B["docker run envoyproxy/envoy --mode validate -c /mounted/config.yaml"]
    B --> C[Validation Result]
```

### Native Validation

```mermaid
graph LR
    A[envoyctl] -->|validate config| B["envoy --mode validate -c /path/to/config.yaml"]
    B --> C[Validation Result]
```

---

## Directory Structure

```mermaid
graph TD
    subgraph "Workspace" ["/var/lib/envoyctl/work/"]
        subgraph "config" ["config/ (Input: Configuration fragments)"]
            subgraph "common" ["common/ (Shared settings)"]
                A["admin.yaml<br/>(Admin interface config)"]
                B["defaults.yaml<br/>(Global defaults)"]
                C["runtime.yaml<br/>(Validation settings)"]
                D["access_log.yaml<br/>(Logging configuration)"]
            end
            subgraph "domains" ["domains/ (Domain definitions)"]
                E["*.yaml<br/>(One file per domain)"]
            end
            subgraph "upstreams" ["upstreams/ (Backend clusters)"]
                F["*.yaml<br/>(One file per upstream)"]
            end
            subgraph "policies" ["policies/ (Reusable policies)"]
                G["headers.yaml<br/>(Header manipulation)"]
                H["ratelimits.yaml<br/>(Rate limiting rules)"]
                I["retries.yaml<br/>(Retry policies)"]
                J["timeouts.yaml<br/>(Timeout configurations)"]
            end
        end
        subgraph "out" ["out/ (Output: Generated config)"]
            K["envoy.generated.yaml<br/>(Complete Envoy configuration)"]
        end
    end
```

---

## Security Model

```mermaid
graph TD
    subgraph "systemd Hardening"
        A[NoNewPrivileges=yes]
        B[ProtectSystem=strict]
        C[ReadWritePaths=/var/lib/envoyctl, /etc/envoy]
        D[PrivateTmp=yes]
    end

    A --> PIPELINE
    B --> PIPELINE
    C --> PIPELINE
    D --> PIPELINE

    subgraph "Validation Pipeline"
        PIPELINE["1. Load fragments (read-only)"]
        PIPELINE --> VALIDATION["2. Semantic validation (in-memory)"]
        VALIDATION --> GENERATE["3. Generate config (write to out/)"]
        GENERATE --> ENVOY_VALIDATION["4. Envoy validation (sandboxed)"]
        ENVOY_VALIDATION --> ATOMIC_INSTALL["5. Atomic install (if validation passes)"]
        ATOMIC_INSTALL --> RESTART["6. Restart Envoy (controlled)"]
    end
```

---

## Error Handling Flow

```mermaid
flowchart TD
    START[Start] --> LOAD[Load Fragments]
    LOAD -->|Parse Error| ERR1[Report: File + Line + Error]
    LOAD -->|Success| VAL[Semantic Validation]
    
    VAL -->|Missing Upstream| ERR2[Report: Domain + Reference]
    VAL -->|Invalid Policy| ERR3[Report: Policy + Issue]
    VAL -->|Success| GEN[Generate Config]
    
    GEN -->|Template Error| ERR4[Report: Generation Failed]
    GEN -->|Success| ENVOY[Envoy Validation]
    
    ENVOY -->|Config Error| ERR5[Report: Envoy Output]
    ENVOY -->|Success| DONE[Continue to Apply/Done]
    
    ERR1 --> EXIT[Exit with Error]
    ERR2 --> EXIT
    ERR3 --> EXIT
    ERR4 --> EXIT
    ERR5 --> EXIT
```

---

## Future Architecture Considerations

### Planned Improvements

1. **Watch Mode**: File system watching for auto-rebuild
2. **Dry Run**: Show what would change without applying
3. **Diff Output**: Compare current vs. generated config
4. **Remote Apply**: Push config to remote Envoy instances
5. **Config Linting**: Additional semantic checks

### Extension Points

- Custom validators (plugin system)
- Alternative output formats (xDS, etc.)
- Multiple Envoy instance support
- Secrets management integration

