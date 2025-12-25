# Configuration Reference

This document provides a complete reference for all configuration options in envoyctl.

---

## Directory Structure

```
config/
├── common/
│   ├── admin.yaml                # Envoy admin interface
│   ├── defaults.yaml             # Global defaults
│   ├── runtime.yaml              # Validation and restart settings
│   ├── access_log.yaml           # Access logging (optional)
│   ├── default_http_backend.yaml # Default HTTP upstream
│   └── default_tls_backend.yaml  # Default TLS passthrough upstream
├── domains/
│   └── <domain>.yaml             # One file per domain
├── upstreams/
│   └── <upstream>.yaml           # One file per backend cluster
└── policies/
    ├── headers.yaml              # Header manipulation rules
    ├── ratelimits.yaml           # Rate limiting configurations
    ├── retries.yaml              # Retry policies
    └── timeouts.yaml             # Timeout configurations
```

---

## Common Configuration

### admin.yaml

Configures the Envoy admin interface.

```yaml
# IP address to bind the admin interface
# Use 127.0.0.1 for local-only access (recommended for production)
address: 0.0.0.0

# Port for the admin interface
port: 9901
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `address` | string | `0.0.0.0` | Bind address for admin interface |
| `port` | integer | `9901` | Port number |

---

### defaults.yaml

Global default values applied to all routes and upstreams.

```yaml
# Default timeout for routes without explicit timeout
route_timeout: 60s

# Default upstream for HTTP traffic (port 80)
http_default_upstream: default_http_backend

# Default upstream for TLS passthrough traffic
tls_passthrough_upstream: default_tls_backend
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `route_timeout` | duration | `60s` | Default route timeout |
| `http_default_upstream` | string | - | Default cluster for HTTP |
| `tls_passthrough_upstream` | string | - | Default cluster for TLS passthrough |

---

### runtime.yaml

Controls how envoyctl validates and restarts Envoy.

```yaml
validate:
  # Validation method: "docker_image" or "native"
  type: docker_image
  
  # Docker image to use for validation (if type: docker_image)
  image: envoyproxy/envoy:v1.31-latest
  
  # Path to envoy binary (if type: native)
  # bin: /usr/bin/envoy

restart:
  # Restart method: "docker_compose", "systemd", or "none"
  type: docker_compose
  
  # Docker Compose settings (if type: docker_compose)
  service: envoy
  file: /opt/envoy/docker-compose.yml
  
  # systemd settings (if type: systemd)
  # unit: envoy.service
```

#### Validate Options

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `docker_image` or `native` |
| `image` | string | Docker image (for docker_image type) |
| `bin` | string | Envoy binary path (for native type) |

#### Restart Options

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `docker_compose`, `systemd`, or `none` |
| `service` | string | Docker Compose service name |
| `file` | string | Docker Compose file path |
| `unit` | string | systemd unit name |

---

## Domain Configuration

Each file in `config/domains/` defines routing for a single domain.

### Full Example

```yaml
# The domain name (required)
domain: example.com

# Routing mode (required)
# Options:
#   - terminate_https_443: Terminate TLS, serve HTTPS on port 443
#   - passthrough_443: Pass TLS traffic through unchanged
#   - http_80: Plain HTTP on port 80
mode: terminate_https_443

# TLS configuration (required if mode is terminate_https_443)
tls:
  # Path to certificate chain file
  cert_chain: /etc/envoy/certs/example.com/fullchain.pem
  
  # Path to private key file
  private_key: /etc/envoy/certs/example.com/privkey.pem

# Route definitions (required, at least one)
routes:
  # Route with prefix matching
  - match: { prefix: "/api/" }
    to_upstream: api_backend
    timeout: 30s
    
  # Route with exact path matching
  - match: { path: "/health" }
    to_upstream: health_backend
    timeout: 5s
    
  # Route with policy references
  - match: { prefix: "/oauth/" }
    to_upstream: oauth_backend
    timeout: 60s
    per_filter_config:
      local_ratelimit: oauth_strict
    
  # Catch-all route (should be last)
  - match: { prefix: "/" }
    to_upstream: default_backend
```

### Route Match Options

| Match Type | Syntax | Description |
|------------|--------|-------------|
| Prefix | `{ prefix: "/api/" }` | Matches paths starting with value |
| Path | `{ path: "/exact" }` | Matches exact path only |
| Regex | `{ regex: "^/v[0-9]+/.*" }` | Matches regex pattern |

### Route Options

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `match` | object | Yes | Path matching rule |
| `to_upstream` | string | Yes | Target upstream name |
| `timeout` | duration | No | Route-specific timeout |
| `per_filter_config` | object | No | Filter-specific settings |

---

## Upstream Configuration

Each file in `config/upstreams/` defines a backend cluster.

### Full Example

```yaml
# Unique name for this upstream (required)
name: my_backend

# Connection timeout to upstream hosts
connect_timeout: 2s

# Service discovery type
# Options: STATIC, STRICT_DNS, LOGICAL_DNS, EDS
type: STRICT_DNS

# Load balancing policy
# Options: ROUND_ROBIN, LEAST_REQUEST, RANDOM, RING_HASH, MAGLEV
lb_policy: ROUND_ROBIN

# Enable HTTP/2 to upstream (optional)
http2: true

# Backend endpoints (required, at least one)
endpoints:
  - address: "backend-1.internal"
    port: 8080
  - address: "backend-2.internal"
    port: 8080
  - address: "10.0.0.100"
    port: 8080

# Health check configuration (optional)
health_check:
  timeout: 5s
  interval: 10s
  unhealthy_threshold: 3
  healthy_threshold: 2
  http:
    path: /health
    expected_statuses: [200]
```

### Upstream Options

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | Yes | - | Unique upstream identifier |
| `connect_timeout` | duration | No | `5s` | Connection timeout |
| `type` | string | No | `STRICT_DNS` | Service discovery type |
| `lb_policy` | string | No | `ROUND_ROBIN` | Load balancing algorithm |
| `http2` | boolean | No | `false` | Use HTTP/2 to upstream |
| `endpoints` | array | Yes | - | List of backend hosts |

### Endpoint Options

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `address` | string | Yes | Hostname or IP address |
| `port` | integer | Yes | Port number |

---

## Policy Configuration

### headers.yaml

Define reusable header manipulation rules.

```yaml
# Request header policies
request_headers:
  # Policy name (referenced by routes)
  add_forwarded_proto:
    add:
      - key: x-forwarded-proto
        value: https
        append: false  # Replace if exists
        
  add_request_id:
    add:
      - key: x-request-id
        value: "%REQ(X-REQUEST-ID)%"
        append: false

# Response header policies
response_headers:
  security_headers:
    add:
      - key: x-content-type-options
        value: nosniff
      - key: x-frame-options
        value: DENY
      - key: x-xss-protection
        value: "1; mode=block"
      - key: strict-transport-security
        value: "max-age=31536000; includeSubDomains"
    remove:
      - server
```

---

### ratelimits.yaml

Define local rate limiting configurations.

```yaml
local_ratelimits:
  # Default rate limit
  default:
    max_tokens: 100      # Maximum tokens in bucket
    tokens_per_fill: 100 # Tokens added per interval
    fill_interval: 1s    # Refill interval
    
  # Strict limit for sensitive endpoints
  strict:
    max_tokens: 10
    tokens_per_fill: 10
    fill_interval: 1s
    
  # Relaxed limit for public APIs
  relaxed:
    max_tokens: 1000
    tokens_per_fill: 1000
    fill_interval: 1s
```

| Field | Type | Description |
|-------|------|-------------|
| `max_tokens` | integer | Maximum tokens in the bucket |
| `tokens_per_fill` | integer | Tokens added each interval |
| `fill_interval` | duration | How often tokens are added |

---

### retries.yaml

Define retry policies.

```yaml
retries:
  # No retries
  none:
    retry_on: []
    num_retries: 0
    
  # Safe for idempotent requests
  safe_idempotent:
    retry_on:
      - 5xx              # Server errors
      - connect-failure  # Connection failed
      - refused-stream   # Stream refused
      - reset            # Connection reset
    num_retries: 2
    per_try_timeout: 2s
    
  # Aggressive retries
  aggressive:
    retry_on:
      - 5xx
      - connect-failure
      - refused-stream
      - retriable-4xx
    num_retries: 5
    per_try_timeout: 5s
```

| Field | Type | Description |
|-------|------|-------------|
| `retry_on` | array | Conditions that trigger retry |
| `num_retries` | integer | Maximum retry attempts |
| `per_try_timeout` | duration | Timeout for each attempt |

---

### timeouts.yaml

Define named timeout values.

```yaml
timeouts:
  default: 60s
  short: 5s
  medium: 30s
  long: 120s
  streaming: 3600s
```

Reference in routes:
```yaml
routes:
  - match: { prefix: "/quick" }
    to_upstream: backend
    timeout: short  # References timeouts.short
```

---

## Duration Format

Durations can be specified as:

| Format | Example | Description |
|--------|---------|-------------|
| Seconds | `30s` | 30 seconds |
| Milliseconds | `500ms` | 500 milliseconds |
| Minutes | `5m` | 5 minutes |
| Hours | `1h` | 1 hour |

---

## Environment Variables

These can be set in `/etc/default/envoyctl`:

| Variable | Default | Description |
|----------|---------|-------------|
| `ENVOYCTL_WORKDIR` | `/var/lib/envoyctl/work` | Workspace root |
| `ENVOYCTL_CONFIG_DIR` | `$WORKDIR/config` | Config fragments directory |
| `ENVOYCTL_OUT_DIR` | `$WORKDIR/out` | Generated output directory |
| `ENVOYCTL_INSTALL_PATH` | `/etc/envoy/envoy.yaml` | Installation target |

---

## CLI Options

```bash
envoyctl [OPTIONS] <COMMAND>

Options:
  --config-dir <PATH>     Config directory [default: config]
  --out-dir <PATH>        Output directory [default: out]
  --install-path <PATH>   Install target [default: /etc/envoy/envoy.yaml]
  --envoy-bin <PATH>      Envoy binary (native mode)
  -h, --help              Print help
  -V, --version           Print version

Commands:
  init       Create a starter workspace
  build      Generate Envoy config from fragments
  validate   Build and validate with Envoy
  apply      Validate, install, and restart Envoy
```

