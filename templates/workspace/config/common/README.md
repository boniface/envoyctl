# Common Configuration

This directory contains **global settings** that apply to the entire Envoy
instance.

These files usually change infrequently.

## Files

| File | Description |
|------|-------------|
| `admin.yaml` | Envoy admin interface (address, port) |
| `defaults.yaml` | Global defaults (timeouts, default upstreams) |
| `access_log.yaml` | Access logging configuration |
| `runtime.yaml` | Validation and restart settings |
| `default_http_backend.yaml` | Default upstream for HTTP traffic |
| `default_tls_backend.yaml` | Default upstream for TLS passthrough |

## Default Upstreams

The `default_http_backend` and `default_tls_backend` upstreams handle traffic
that doesn't match any specific domain configuration:

- **default_http_backend**: Catch-all for HTTP (port 80) traffic
- **default_tls_backend**: Catch-all for TLS passthrough (port 443) traffic

Edit these files to point to your actual backend services.

⚠️ **Warning**: Changes here affect the whole proxy.
