# Data Flow

This document describes how configuration data flows through envoyctl from input fragments to the final Envoy configuration.

---

## Overview

```mermaid
flowchart LR
    subgraph Input["Input Fragments"]
        D[domains/*.yaml]
        U[upstreams/*.yaml]
        P[policies/*.yaml]
        C[common/*.yaml]
    end

    subgraph Processing["Processing"]
        L[Load & Parse]
        V[Validate]
        G[Generate]
    end

    subgraph Output["Output"]
        O["envoy.generated.yaml"]
    end

    D --> L
    U --> L
    P --> L
    C --> L
    L --> V
    V --> G
    G --> O
    O -->|apply| I
```

---

## Detailed Data Flow

### Stage 1: Loading

```mermaid
graph LR
    subgraph "Input Files"
        A["config/common/defaults.yaml"]
        B["config/common/admin.yaml"]
        C["config/common/runtime.yaml"]
        D["config/domains/*.yaml"]
        E["config/upstreams/*.yaml"]
        F["config/policies/*.yaml"]
    end

    subgraph "load.rs Processing"
        G["Load & Parse"]
    end

    subgraph "Output Structure"
        H["LoadedConfig {<br/>  defaults: DefaultsSpec,<br/>  admin: AdminSpec,<br/>  runtime: RuntimeSpec,<br/>  domains: Vec&lt;DomainSpec&gt;,<br/>  upstreams: Vec&lt;UpstreamSpec&gt;,<br/>  policies: PolicyBundle<br/>}"]
    end

    A --> G
    B --> G
    C --> G
    D --> G
    E --> G
    F --> G
    G --> H
```

### Stage 2: Validation

```mermaid
graph TB
    A["LoadedConfig"]
    A --> B["validate.rs Processing"]

    subgraph "Validation Checks"
        C["✓ All domain.to_upstream references exist in upstreams"]
        D["✓ All policy references exist"]
        E["✓ TLS cert paths specified for terminate mode"]
        F["✓ Required fields present and valid"]
        G["✓ No duplicate domain names"]
        H["✓ No duplicate upstream names"]
    end

    B --> C
    B --> D
    B --> E
    B --> F
    B --> G
    B --> H

    C --> I["Result<(), ValidationError>"]
    D --> I
    E --> I
    F --> I
    G --> I
    H --> I
```

### Stage 3: Generation

```mermaid
graph TB
    A["LoadedConfig"]
    A --> B["generate.rs Processing"]

    subgraph "Build Components"
        C["Build Listeners"]
        D["Build Clusters"]
    end

    subgraph "Listener Details"
        C1["• HTTP :80"]
        C2["• HTTPS :443"]
        C3["• TLS terminate"]
        C4["• TLS passthrough"]
    end

    subgraph "Cluster Details"
        D1["• From upstreams"]
        D2["• Endpoints"]
        D3["• Health checks"]
        D4["• Timeouts"]
    end

    B --> C
    B --> D
    C --> C1
    C --> C2
    C --> C3
    C --> C4
    D --> D1
    D --> D2
    D --> D3
    D --> D4

    C --> E["Envoy Config (YAML)"]
    D --> E

    subgraph "Generated YAML Structure"
        F["• admin: address, port"]
        G["• static_resources: listeners, clusters"]
        H["• listeners: http, https, tls"]
        I["• clusters: endpoints, load assignment"]
    end

    E --> F
    E --> G
    E --> H
    E --> I
```

---

## Fragment to Envoy Mapping

### Domain → Listener + Routes

```yaml
# Input: config/domains/example.com.yaml
domain: example.com
mode: terminate_https_443
tls:
  cert_chain: /etc/envoy/certs/example.com/fullchain.pem
  private_key: /etc/envoy/certs/example.com/privkey.pem
routes:
  - match: { prefix: "/" }
    to_upstream: backend
```

```yaml
# Output: Part of envoy.generated.yaml
static_resources:
  listeners:
    - name: https_listener
      address:
        socket_address: { address: 0.0.0.0, port_value: 443 }
      listener_filters:
        - name: tls_inspector
      filter_chains:
        - filter_chain_match:
            server_names: ["example.com"]
          transport_socket:
            name: envoy.transport_sockets.tls
            typed_config:
              "@type": type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.DownstreamTlsContext
              common_tls_context:
                tls_certificates:
                  - certificate_chain: { filename: /etc/envoy/certs/example.com/fullchain.pem }
                    private_key: { filename: /etc/envoy/certs/example.com/privkey.pem }
          filters:
            - name: envoy.filters.network.http_connection_manager
              typed_config:
                route_config:
                  virtual_hosts:
                    - name: example.com
                      domains: ["example.com"]
                      routes:
                        - match: { prefix: "/" }
                          route: { cluster: backend }
```

### Upstream → Cluster

```yaml
# Input: config/upstreams/backend.yaml
name: backend
connect_timeout: 2s
type: STRICT_DNS
lb_policy: ROUND_ROBIN
http2: true
endpoints:
  - { address: "10.0.0.1", port: 8080 }
  - { address: "10.0.0.2", port: 8080 }
```

```yaml
# Output: Part of envoy.generated.yaml
static_resources:
  clusters:
    - name: backend
      connect_timeout: 2s
      type: STRICT_DNS
      lb_policy: ROUND_ROBIN
      typed_extension_protocol_options:
        envoy.extensions.upstreams.http.v3.HttpProtocolOptions:
          "@type": type.googleapis.com/envoy.extensions.upstreams.http.v3.HttpProtocolOptions
          explicit_http_config:
            http2_protocol_options: {}
      load_assignment:
        cluster_name: backend
        endpoints:
          - lb_endpoints:
              - endpoint:
                  address:
                    socket_address: { address: "10.0.0.1", port_value: 8080 }
              - endpoint:
                  address:
                    socket_address: { address: "10.0.0.2", port_value: 8080 }
```

---

## Command Flow

### `envoyctl build`

```mermaid
sequenceDiagram
    participant CLI
    participant Load
    participant Validate
    participant Generate
    participant FS as File System

    CLI->>Load: load_all(config_dir)
    Load->>FS: Read YAML files
    FS-->>Load: File contents
    Load-->>CLI: LoadedConfig
    
    CLI->>Validate: validate_model(config)
    Validate-->>CLI: Ok(()) or Error
    
    CLI->>Generate: generate_envoy_yaml(config)
    Generate-->>CLI: YAML Value
    
    CLI->>FS: Write out/envoy.generated.yaml
    FS-->>CLI: Ok
```

### `envoyctl validate`

```mermaid
sequenceDiagram
    participant CLI
    participant Build
    participant Exec
    participant Docker

    CLI->>Build: cmd_build(cli)
    Build-->>CLI: Ok (config written)
    
    CLI->>Exec: run_envoy_validate(config_path)
    Exec->>Docker: docker run envoyproxy/envoy --mode validate
    Docker-->>Exec: Exit code + output
    Exec-->>CLI: Ok or Error
```

### `envoyctl apply`

```mermaid
sequenceDiagram
    participant CLI
    participant Validate
    participant FS as File System
    participant Exec
    participant Envoy

    CLI->>Validate: cmd_validate(cli)
    Validate-->>CLI: Ok
    
    CLI->>FS: atomic_install(src, dst)
    Note over FS: Write to tmp, rename atomically
    FS-->>CLI: Ok
    
    CLI->>Exec: restart_envoy()
    Exec->>Envoy: docker-compose restart / systemctl restart
    Envoy-->>Exec: Ok
    Exec-->>CLI: Ok
```

---

## Error Propagation

```mermaid
graph TB
    subgraph "Load Error"
        A1["File not found"]
        A2["Parse error"]
        A3["Missing field"]
    end

    subgraph "Validation Error"
        B1["Unknown upstream"]
        B2["Unknown policy"]
        B3["Duplicate name"]
    end

    subgraph "Generation Error"
        C1["Serialization"]
    end

    subgraph "Envoy Validation Error"
        D1["Config invalid"]
    end

    subgraph "Apply Error"
        E1["Install failed"]
        E2["Restart failed"]
    end

    A1 --> A_out["'could not read config/domains/foo.yaml'"]
    A2 --> A_out2["'YAML error at line 5: expected string'"]
    A3 --> A_out3["'domain.yaml: missing required field domain'"]

    B1 --> B_out["'domain x.com references unknown upstream y'"]
    B2 --> B_out2["'route references unknown rate limit z'"]
    B3 --> B_out3["'duplicate upstream name: backend'"]

    C1 --> C_out["'failed to serialize envoy config'"]

    D1 --> D_out["'envoy validation failed: [envoy output]'"]

    E1 --> E_out["'could not write to /etc/envoy/envoy.yaml'"]
    E2 --> E_out2["'docker-compose restart failed: [output]'"]

    A_out --> ErrorHandling["Error Handling with anyhow::Result"]
    A_out2 --> ErrorHandling
    A_out3 --> ErrorHandling
    B_out --> ErrorHandling
    B_out2 --> ErrorHandling
    B_out3 --> ErrorHandling
    C_out --> ErrorHandling
    D_out --> ErrorHandling
    E_out --> ErrorHandling
    E_out2 --> ErrorHandling
```

All errors include context and are propagated up using `anyhow::Result` with `.context()` for clear error messages.

