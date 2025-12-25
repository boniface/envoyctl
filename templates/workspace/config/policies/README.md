# Policies

Policies are **named, reusable behavior blocks** that routes can reference.

They prevent duplication and ensure consistent behavior across your domains.

## Policy Types

| File | Description |
|------|-------------|
| `ratelimits.yaml` | Local (per-proxy) HTTP rate limits |
| `timeouts.yaml` | Named timeout presets |
| `headers.yaml` | Request and response header rules |
| `retries.yaml` | Retry behavior for failed requests |

## Usage

Routes reference policies by name in the domain configuration:

```yaml
routes:
  - match: { prefix: "/api/" }
    to_upstream: api_backend
    timeout: medium                    # References timeouts.yaml
    per_filter_config:
      local_ratelimit: strict          # References ratelimits.yaml
```

## Available Rate Limits

- `default` - 100 req/s (general traffic)
- `strict` - 30 req/s (sensitive endpoints like login)
- `moderate` - 60 req/s (balanced protection)
- `relaxed` - 300 req/s (high-traffic public APIs)

## Available Timeouts

- `default` - 60s
- `short` - 5s
- `medium` - 30s
- `long` - 120s

## Validation

If a policy is missing or misspelled, envoyctl validation will fail with a
clear error message indicating which policy reference is invalid.
