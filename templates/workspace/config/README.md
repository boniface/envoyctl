# Configuration Fragments

This directory contains **small, focused YAML files** that describe parts
of your Envoy configuration.

Each subdirectory owns a specific concern.

## Directory Structure

| Directory | Purpose |
|-----------|---------|
| `common/` | Global settings, defaults, admin interface |
| `domains/` | One file per domain/hostname you want to route |
| `upstreams/` | One file per backend service |
| `policies/` | Reusable policies (rate limits, headers, retries) |

## Quick Start

1. **Edit `domains/example.com.yaml`** - Configure your first domain
2. **Edit `upstreams/api_backend.yaml`** - Point to your backend service
3. **Run `envoyctl validate`** - Check your configuration
4. **Run `envoyctl apply`** - Deploy to Envoy

## How It Works

```
config/                          envoyctl build
├── common/defaults.yaml    ──────────────────►  out/envoy.generated.yaml
├── domains/example.com.yaml                          │
├── upstreams/api_backend.yaml                        │
└── policies/ratelimits.yaml                          ▼
                                              Envoy loads config
```

All files are validated before Envoy is restarted.
