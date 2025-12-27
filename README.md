# envoyctl

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tests](https://github.com/boniface/envoyctl/actions/workflows/tests.yml/badge.svg)](https://github.com/boniface/envoyctl/actions/workflows/tests.yml)

**envoyctl** is a configuration management tool for [Envoy Proxy](https://www.envoyproxy.io/).

Manage complex Envoy configurations using **small, focused YAML fragments**, validate with Envoy itself, and deploy with confidence.

---

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [What is Envoy?](#what-is-envoy)
- [Why envoyctl?](#why-envoyctl)
- [Features](#features)
- [Workspace Layout](#workspace-layout)
- [Configuration Examples](#configuration-examples)
- [Commands](#commands)
- [Validation Modes](#validation-modes)
- [TLS Behavior](#tls-behavior)
- [Docker Deployment](#docker-deployment)
- [Contributing](#contributing)
- [License](#license)

---

## Installation

### From APT Repository (Debian/Ubuntu) — Recommended

```bash
# 1. Download and install the GPG signing key
sudo mkdir -p /etc/apt/keyrings
curl -fsSL https://boniface.github.io/envoyctl/public.gpg \
  | sudo gpg --dearmor -o /etc/apt/keyrings/envoyctl.gpg

# 2. Add the APT repository
echo "deb [signed-by=/etc/apt/keyrings/envoyctl.gpg] https://boniface.github.io/envoyctl stable main" \
  | sudo tee /etc/apt/sources.list.d/envoyctl.list

# 3. Update and install
sudo apt update
sudo apt install envoyctl

# 4. Verify installation
envoyctl --version
```

### From Source (Rust)

```bash
# Clone the repository
git clone https://github.com/boniface/envoyctl.git
cd envoyctl

# Build release binary
cargo build --release

# Install to system
sudo cp target/release/envoyctl /usr/local/bin/

# Verify installation
envoyctl --version
```

### From GitHub Releases

Download the `.deb` package directly from [GitHub Releases](https://github.com/boniface/envoyctl/releases):

```bash
# Download the latest release
curl -LO https://github.com/boniface/envoyctl/releases/latest/download/envoyctl_0.1.0_amd64.deb

# Install
sudo dpkg -i envoyctl_*.deb

# Verify
envoyctl --version
```

---

## Quick Start

### 1. Initialize a Workspace

```bash
envoyctl init --dir ./my-envoy-config
cd ./my-envoy-config
```

### 2. Review and Edit Configuration

```bash
# Edit the example domain
vim config/domains/example.com.yaml

# Edit backend upstreams
vim config/upstreams/api_backend.yaml
vim config/upstreams/web_frontend.yaml

# Configure validation mode
vim config/common/runtime.yaml
```

### 3. Build Configuration

```bash
envoyctl --config-dir ./config --out-dir ./out build
```

### 4. Validate with Envoy

```bash
envoyctl --config-dir ./config --out-dir ./out validate
```

The generated configuration will be at `./out/envoy.generated.yaml`.

---

## What is Envoy?

[Envoy](https://www.envoyproxy.io/) is a high-performance, open-source edge and service proxy designed for cloud-native applications. Originally built at Lyft, it's now a graduated [CNCF](https://www.cncf.io/) project.

| Capability | Description |
|------------|-------------|
| **Load Balancing** | Distributes traffic across backend services |
| **TLS Termination** | Handles HTTPS at the edge |
| **TLS Passthrough** | Routes encrypted traffic via SNI |
| **Reverse Proxy** | Routes HTTP/HTTPS to backends |
| **Rate Limiting** | Protects services from abuse |
| **Health Checking** | Monitors backend health |
| **Observability** | Metrics, logging, and tracing |

### The Configuration Challenge

Envoy is powerful but verbose. A simple route requires 50+ lines of YAML:

```yaml
static_resources:
  listeners:
    - name: listener_0
      address:
        socket_address:
          address: 0.0.0.0
          port_value: 443
      filter_chains:
        - filters:
            - name: envoy.filters.network.http_connection_manager
              typed_config:
                "@type": type.googleapis.com/...
                # ... continues for many more lines
```

**envoyctl solves this** by generating complete Envoy configurations from simple fragments.

---

## Why envoyctl?

| Problem | envoyctl Solution |
|---------|-------------------|
| Giant, unwieldy config files | Small, focused YAML fragments |
| Hard to make changes safely | One file per domain/upstream |
| Late or no validation | Always validated before use |
| Risky deployments | Atomic config generation |
| Difficult to audit | Deterministic, commented output |

### Simple Configuration

Instead of hundreds of lines, write this:

```yaml
# config/domains/example.com.yaml
domain: example.com
mode: terminate_https_443
tls:
  cert_chain: /etc/envoy/certs/example.com/fullchain.pem
  private_key: /etc/envoy/certs/example.com/privkey.pem
routes:
  - match: { prefix: "/api/" }
    to_upstream: api_backend
  - match: { prefix: "/" }
    to_upstream: web_frontend
```

```yaml
# config/upstreams/api_backend.yaml
name: api_backend
connect_timeout: 2s
type: STRICT_DNS
lb_policy: ROUND_ROBIN
endpoints:
  - { address: "api-service", port: 8080 }
```

Then generate:

```bash
envoyctl --config-dir ./config --out-dir ./out build
```

---

## Features

- ✅ **Fragment-based configuration** — Organize by domain, upstream, policy
- ✅ **Envoy v3 static configuration** — Full compatibility
- ✅ **TLS termination & passthrough** — Flexible HTTPS handling
- ✅ **Built-in validation** — Validate with Docker or native Envoy
- ✅ **Section comments** — Generated YAML includes helpful comments
- ✅ **Timestamps** — Know when config was generated
- ✅ **APT repository** — Easy installation on Debian/Ubuntu

---

## Workspace Layout

```
my-envoy-config/
├── config/
│   ├── common/
│   │   ├── admin.yaml              # Envoy admin interface
│   │   ├── defaults.yaml           # Global defaults
│   │   ├── runtime.yaml            # Validation settings
│   │   ├── access_log.yaml         # Logging configuration
│   │   ├── default_http_backend.yaml
│   │   └── default_tls_backend.yaml
│   ├── domains/
│   │   └── example.com.yaml        # One file per domain
│   ├── upstreams/
│   │   ├── api_backend.yaml        # One file per backend
│   │   └── web_frontend.yaml
│   └── policies/
│       ├── headers.yaml            # Header manipulation
│       ├── ratelimits.yaml         # Rate limiting
│       ├── retries.yaml            # Retry policies
│       └── timeouts.yaml           # Timeout configs
└── out/
    └── envoy.generated.yaml        # Generated config (don't edit!)
```

---

## Configuration Examples

### Adding a Domain

Create `config/domains/api.example.com.yaml`:

```yaml
domain: api.example.com
mode: terminate_https_443

tls:
  cert_chain: /etc/envoy/certs/api.example.com/fullchain.pem
  private_key: /etc/envoy/certs/api.example.com/privkey.pem

routes:
  - match: { prefix: "/v1/" }
    to_upstream: api_v1
    timeout: 30s
    
  - match: { prefix: "/" }
    to_upstream: web_frontend
    timeout: 60s
```

### Adding an Upstream

Create `config/upstreams/api_v1.yaml`:

```yaml
name: api_v1
connect_timeout: 2s
type: STRICT_DNS
lb_policy: ROUND_ROBIN
http2: true
endpoints:
  - { address: "api-v1-service", port: 8080 }
  - { address: "api-v1-backup", port: 8080 }
```

### Rate Limiting

In `config/policies/ratelimits.yaml`:

```yaml
local_ratelimits:
  strict:
    max_tokens: 30
    tokens_per_fill: 30
    fill_interval: 1s
```

Reference in domain routes:

```yaml
routes:
  - match: { prefix: "/login" }
    to_upstream: auth_service
    per_filter_config:
      local_ratelimit: strict
```

---

## Commands

| Command | Description |
|---------|-------------|
| `envoyctl init --dir <path>` | Create a new workspace from templates |
| `envoyctl build` | Generate Envoy config from fragments |
| `envoyctl validate` | Build + validate with Envoy |

### Options

```bash
envoyctl [OPTIONS] <COMMAND>

Options:
  --config-dir <PATH>     Config directory [default: config]
  --out-dir <PATH>        Output directory [default: out]
  --install-path <PATH>   Install target [default: /etc/envoy/envoy.yaml]
  --envoy-bin <PATH>      Envoy binary path (for native validation)
  -h, --help              Print help
  -V, --version           Print version
```

### Examples

```bash
# Build with default paths
envoyctl build

# Build with custom paths
envoyctl --config-dir ./my-config --out-dir ./my-output build

# Validate configuration
envoyctl --config-dir ./config --out-dir ./out validate
```

---

## Validation Modes

Configure in `config/common/runtime.yaml`:

### Docker Exec (for Docker deployments)

Validates inside a running Envoy container:

```yaml
validate:
  type: docker_exec
  container: envoy                    # Container name
  config_path: /etc/envoy/envoy.yaml  # Path inside container
```

### Native (for baremetal/systemd)

Validates using local Envoy binary with sudo:

```yaml
validate:
  type: native
  user: envoy                         # User to run as
  bin: envoy                          # Envoy binary path
  config_path: /etc/envoy/envoy.yaml  # Config path
```

### Docker Image (for CI/testing)

Validates using a fresh container:

```yaml
validate:
  type: docker_image
  image: envoyproxy/envoy:v1.31-latest
```

---

## TLS Behavior

| Mode | Port | Description |
|------|------|-------------|
| `terminate_https_443` | 443 | Envoy terminates TLS (requires cert/key) |
| `passthrough_443` | 443 | TLS passed through unchanged (SNI routing) |
| `http_80` | 80 | Plain HTTP |

**Default for unknown domains on :443**: TLS passthrough to `default_tls_backend`.

---

## Docker Deployment

See the full [Docker Deployment Tutorial](docs/docker-deployment.md).

### Quick Example

```bash
# Generate configuration
envoyctl --config-dir ./config --out-dir ./out build

# Run Envoy with generated config
docker run -d \
  --name envoy \
  -p 80:80 -p 443:443 -p 9901:9901 \
  -v $(pwd)/out/envoy.generated.yaml:/etc/envoy/envoy.yaml:ro \
  -v /etc/envoy/certs:/etc/envoy/certs:ro \
  envoyproxy/envoy:v1.31-latest
```

### Docker Compose

```yaml
version: '3.8'
services:
  envoy:
    image: envoyproxy/envoy:v1.31-latest
    ports:
      - "80:80"
      - "443:443"
      - "127.0.0.1:9901:9901"
    volumes:
      - ./out/envoy.generated.yaml:/etc/envoy/envoy.yaml:ro
      - /etc/letsencrypt:/etc/envoy/certs:ro
    restart: unless-stopped
```

---

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# Clone
git clone https://github.com/boniface/envoyctl.git
cd envoyctl

# Build
cargo build

# Run tests
cargo test

# Check formatting
cargo fmt -- --check

# Run lints
cargo clippy -- -D warnings
```

---

## License

MIT License — Copyright (c) 2025 Boniface Kabaso

See [LICENSE](./LICENSE) for full text.

---

## Links

- **Repository**: [github.com/boniface/envoyctl](https://github.com/boniface/envoyctl)
- **APT Repository**: [boniface.github.io/envoyctl](https://boniface.github.io/envoyctl)
- **Documentation**: [docs/](./docs/)
- **Envoy Proxy**: [envoyproxy.io](https://www.envoyproxy.io/)

