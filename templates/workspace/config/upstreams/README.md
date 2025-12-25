# Upstreams

Upstreams describe **backend services** that Envoy can route traffic to.

Each upstream maps to an Envoy cluster.

## One File Per Upstream

Create one YAML file per backend service:

| File | Description |
|------|-------------|
| `api_backend.yaml` | Your API/backend service |
| `web_frontend.yaml` | Your web frontend/UI service |
| `database_proxy.yaml` | Database connection proxy |
| `auth_service.yaml` | Authentication service |

## What an Upstream Defines

- **name**: Unique identifier (referenced by domain routes)
- **connect_timeout**: How long to wait for connection
- **type**: Service discovery (STATIC, STRICT_DNS, etc.)
- **lb_policy**: Load balancing algorithm
- **endpoints**: List of backend servers

## Example

```yaml
name: my_service
connect_timeout: 2s
type: STRICT_DNS
lb_policy: ROUND_ROBIN
endpoints:
  - { address: "service-host", port: 8080 }
```

## Tips

- Upstreams are reusable across multiple domains
- Use descriptive names: `api_backend`, not `cluster1`
- Start with one endpoint, add more for high availability
