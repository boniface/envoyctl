# Domains

This directory contains **one YAML file per domain/hostname**.

Each file defines:
- How traffic for the domain is handled
- Whether TLS is terminated or passed through
- Routing rules to upstreams
- Optional per-route policies (rate limits, timeouts)

## Naming Convention

File name should match the domain name:

| Domain | File Name |
|--------|-----------|
| example.com | `example.com.yaml` |
| api.example.com | `api.example.com.yaml` |
| app.mycompany.io | `app.mycompany.io.yaml` |

## Routing Modes

| Mode | Description |
|------|-------------|
| `terminate_https_443` | Terminate TLS at Envoy (requires cert/key) |
| `passthrough_443` | Pass TLS through unchanged (SNI routing) |
| `http_80` | Plain HTTP on port 80 |

## Default Behavior

If a domain is **not** listed here:
- HTTP traffic goes to `default_http_backend` (see common/defaults.yaml)
- TLS traffic is passed through to `default_tls_backend`

## Getting Started

1. Copy `example.com.yaml` to `<your-domain>.yaml`
2. Update the `domain` field
3. Configure TLS certificate paths (if terminating)
4. Define your routes
5. Run `envoyctl validate` to check your configuration
