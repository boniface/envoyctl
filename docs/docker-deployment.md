# Docker Deployment Tutorial

This tutorial shows you how to deploy Envoy using envoyctl and Docker containers.

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Overview](#overview)
- [Step 1: Initialize a Workspace](#step-1-initialize-a-workspace)
- [Step 2: Configure Your Domain](#step-2-configure-your-domain)
- [Step 3: Configure Your Backend](#step-3-configure-your-backend)
- [Step 4: Configure Validation Settings](#step-4-configure-validation-settings)
- [Step 5: Generate and Validate](#step-5-generate-and-validate)
- [Step 6: Run Envoy with Docker](#step-6-run-envoy-with-docker)
- [Step 7: Docker Compose Setup](#step-7-docker-compose-setup)
- [Step 8: TLS Certificates](#step-8-tls-certificates)
- [Step 9: Production Considerations](#step-9-production-considerations)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

Before starting, ensure you have:

- **envoyctl** installed ([Installation Guide](../README.md#installation))
- **Docker** installed and running
- Basic understanding of Envoy concepts
- (Optional) TLS certificates for HTTPS

Verify your setup:

```bash
# Check envoyctl
envoyctl --version

# Check Docker
docker --version
docker run --rm envoyproxy/envoy:v1.31-latest --version
```

---

## Overview

The deployment workflow:

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  YAML Fragments │────▶│  envoyctl build │────▶│ envoy.generated │
│  (you edit)     │     │                 │     │     .yaml       │
└─────────────────┘     └─────────────────┘     └────────┬────────┘
                                                         │
                        ┌─────────────────┐              │
                        │  Docker Envoy   │◀─────────────┘
                        │  Container      │   (mount as volume)
                        └─────────────────┘
```

---

## Step 1: Initialize a Workspace

Create a new envoyctl workspace:

```bash
# Create workspace
envoyctl init --dir ./envoy-docker-demo
cd ./envoy-docker-demo

# View the structure
tree .
```

You should see:

```
envoy-docker-demo/
├── config/
│   ├── common/
│   │   ├── admin.yaml
│   │   ├── defaults.yaml
│   │   ├── runtime.yaml
│   │   └── ...
│   ├── domains/
│   │   └── example.com.yaml
│   ├── upstreams/
│   │   ├── api_backend.yaml
│   │   └── web_frontend.yaml
│   └── policies/
│       └── ...
└── out/
    └── (generated files go here)
```

---

## Step 2: Configure Your Domain

Edit `config/domains/example.com.yaml` to match your domain:

```yaml
# config/domains/example.com.yaml

# Your domain name
domain: example.com

# For this tutorial, we'll start with HTTP only
# Change to 'terminate_https_443' when you have TLS certs
mode: http_80

# Routes - map URL paths to backend services
routes:
  # API routes go to the API backend
  - match: { prefix: "/api/" }
    to_upstream: api_backend
    timeout: 30s

  # Health check endpoint
  - match: { path: "/health" }
    to_upstream: api_backend
    timeout: 5s

  # Everything else goes to the web frontend
  - match: { prefix: "/" }
    to_upstream: web_frontend
    timeout: 60s
```

---

## Step 3: Configure Your Backend

Edit the upstream configurations to point to your actual services.

### API Backend

Edit `config/upstreams/api_backend.yaml`:

```yaml
# config/upstreams/api_backend.yaml

name: api_backend
connect_timeout: 2s
type: STRICT_DNS
lb_policy: ROUND_ROBIN

# For Docker networking, use the container/service name
# Examples:
#   - Docker Compose service: "api" 
#   - Docker network container: "my-api-container"
#   - Host machine: "host.docker.internal" (Docker Desktop)
#   - External: "api.internal.example.com"
endpoints:
  - { address: "api", port: 3000 }
```

### Web Frontend

Edit `config/upstreams/web_frontend.yaml`:

```yaml
# config/upstreams/web_frontend.yaml

name: web_frontend
connect_timeout: 2s
type: STRICT_DNS
lb_policy: ROUND_ROBIN

endpoints:
  - { address: "frontend", port: 8080 }
```

---

## Step 4: Configure Validation Settings

Edit `config/common/runtime.yaml` to use Docker for validation:

```yaml
# config/common/runtime.yaml

# Use Docker for validation (recommended)
validate:
  type: docker_image
  image: envoyproxy/envoy:v1.31-latest

# For Docker deployments, we'll manage restarts ourselves
restart:
  type: none
```

---

## Step 5: Generate and Validate

Now generate and validate your configuration:

```bash
# Build the configuration
envoyctl build --config-dir ./config --out-dir ./out

# Validate with Envoy
envoyctl validate --config-dir ./config --out-dir ./out
```

If successful, you'll see:

```
Wrote out/envoy.generated.yaml
Validation OK
```

Check the generated file:

```bash
# View the generated Envoy configuration
cat out/envoy.generated.yaml
```

---

## Step 6: Run Envoy with Docker

### Simple Docker Run

Run Envoy with your generated configuration:

```bash
# HTTP only (port 80)
docker run -d \
  --name envoy \
  -p 80:80 \
  -p 9901:9901 \
  -v $(pwd)/out/envoy.generated.yaml:/etc/envoy/envoy.yaml:ro \
  envoyproxy/envoy:v1.31-latest

# Check logs
docker logs envoy

# Check it's working
curl http://localhost/health
```

### With Host Networking

If your backends are on the host machine:

```bash
docker run -d \
  --name envoy \
  --network host \
  -v $(pwd)/out/envoy.generated.yaml:/etc/envoy/envoy.yaml:ro \
  envoyproxy/envoy:v1.31-latest
```

### Stop and Remove

```bash
docker stop envoy
docker rm envoy
```

---

## Step 7: Docker Compose Setup

For production, use Docker Compose. Create a `docker-compose.yml`:

```yaml
# docker-compose.yml

version: '3.8'

services:
  # Envoy Proxy
  envoy:
    image: envoyproxy/envoy:v1.31-latest
    container_name: envoy
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
      - "9901:9901"  # Admin interface (restrict in production!)
    volumes:
      - ./out/envoy.generated.yaml:/etc/envoy/envoy.yaml:ro
      - ./certs:/etc/envoy/certs:ro  # TLS certificates
    networks:
      - app-network
    depends_on:
      - api
      - frontend

  # Example API service
  api:
    image: your-api-image:latest
    container_name: api
    restart: unless-stopped
    expose:
      - "3000"
    networks:
      - app-network
    environment:
      - NODE_ENV=production

  # Example frontend service
  frontend:
    image: nginx:alpine
    container_name: frontend
    restart: unless-stopped
    expose:
      - "8080"
    volumes:
      - ./frontend/dist:/usr/share/nginx/html:ro
    networks:
      - app-network

networks:
  app-network:
    driver: bridge
```

### Run with Docker Compose

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f envoy

# Check Envoy admin interface
curl http://localhost:9901/stats

# Reload Envoy after config changes
docker-compose restart envoy
```

### Update Configuration Workflow

When you make configuration changes:

```bash
# 1. Edit your fragments
vim config/domains/example.com.yaml

# 2. Regenerate and validate
envoyctl validate --config-dir ./config --out-dir ./out

# 3. Restart Envoy to pick up changes
docker-compose restart envoy
```

---

## Step 8: TLS Certificates

### Option A: Let's Encrypt with Certbot

```bash
# Install certbot
sudo apt install certbot

# Get certificates
sudo certbot certonly --standalone -d example.com

# Certificates are saved to:
# /etc/letsencrypt/live/example.com/fullchain.pem
# /etc/letsencrypt/live/example.com/privkey.pem
```

### Update Domain Configuration

Edit `config/domains/example.com.yaml`:

```yaml
domain: example.com
mode: terminate_https_443  # Enable TLS termination

tls:
  cert_chain: /etc/envoy/certs/example.com/fullchain.pem
  private_key: /etc/envoy/certs/example.com/privkey.pem

routes:
  - match: { prefix: "/" }
    to_upstream: web_frontend
```

### Update Docker Compose

```yaml
services:
  envoy:
    volumes:
      - ./out/envoy.generated.yaml:/etc/envoy/envoy.yaml:ro
      # Mount Let's Encrypt certificates
      - /etc/letsencrypt/live/example.com:/etc/envoy/certs/example.com:ro
      - /etc/letsencrypt/archive/example.com:/etc/letsencrypt/archive/example.com:ro
```

### Option B: Self-Signed Certificates (Development)

```bash
# Create certs directory
mkdir -p certs/example.com

# Generate self-signed certificate
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout certs/example.com/privkey.pem \
  -out certs/example.com/fullchain.pem \
  -subj "/CN=example.com"
```

---

## Step 9: Production Considerations

### Security Checklist

- [ ] **Restrict admin interface**: Don't expose port 9901 publicly
- [ ] **Use TLS**: Always terminate TLS in production
- [ ] **Rate limiting**: Add rate limits to protect against abuse
- [ ] **Health checks**: Configure upstream health checks
- [ ] **Logging**: Enable access logging for debugging

### Restrict Admin Interface

For production, only expose admin locally:

```yaml
# docker-compose.yml
services:
  envoy:
    ports:
      - "80:80"
      - "443:443"
      - "127.0.0.1:9901:9901"  # Only accessible from localhost
```

### Add Health Checks

Edit `config/upstreams/api_backend.yaml`:

```yaml
name: api_backend
connect_timeout: 2s
type: STRICT_DNS
lb_policy: ROUND_ROBIN
endpoints:
  - { address: "api", port: 3000 }

# Uncomment when supported
# health_check:
#   timeout: 5s
#   interval: 10s
#   unhealthy_threshold: 3
#   healthy_threshold: 2
#   http:
#     path: /health
#     expected_statuses: [200]
```

### Enable Rate Limiting

Add to `config/policies/ratelimits.yaml`:

```yaml
local_ratelimits:
  default:
    max_tokens: 100
    tokens_per_fill: 100
    fill_interval: 1s
    
  login:
    max_tokens: 10
    tokens_per_fill: 10
    fill_interval: 1s
```

Apply to routes:

```yaml
routes:
  - match: { path: "/login" }
    to_upstream: api_backend
    per_filter_config:
      local_ratelimit: login
```

### Automated Config Reload Script

Create a script for safe config updates:

```bash
#!/bin/bash
# reload-envoy.sh

set -e

echo "Validating configuration..."
envoyctl validate --config-dir ./config --out-dir ./out

echo "Restarting Envoy..."
docker-compose restart envoy

echo "Waiting for Envoy to be ready..."
sleep 2

echo "Checking health..."
curl -s http://localhost:9901/ready && echo " Envoy is ready!"
```

Make it executable:

```bash
chmod +x reload-envoy.sh
./reload-envoy.sh
```

---

## Troubleshooting

### Envoy Won't Start

```bash
# Check logs
docker logs envoy

# Validate configuration manually
docker run --rm \
  -v $(pwd)/out/envoy.generated.yaml:/etc/envoy/envoy.yaml:ro \
  envoyproxy/envoy:v1.31-latest \
  --mode validate -c /etc/envoy/envoy.yaml
```

### Connection Refused to Backend

```bash
# Check if containers are on the same network
docker network ls
docker network inspect app-network

# Test connectivity from Envoy container
docker exec envoy curl http://api:3000/health
```

### Certificate Errors

```bash
# Verify certificate paths
docker exec envoy ls -la /etc/envoy/certs/

# Check certificate validity
openssl x509 -in certs/example.com/fullchain.pem -text -noout
```

### Check Envoy Clusters

```bash
# View cluster status
curl http://localhost:9901/clusters

# View configuration
curl http://localhost:9901/config_dump
```

### View Access Logs

```bash
# Follow Envoy logs
docker logs -f envoy
```

---

## Next Steps

- Read the [Architecture Documentation](./architecture.md)
- Explore [Configuration Reference](./configuration.md)
- Set up [systemd integration](../README.md#systemd-integration) for bare-metal deployments

---

## Complete Docker Compose Example

Here's a full production-ready example:

```yaml
# docker-compose.yml

version: '3.8'

services:
  envoy:
    image: envoyproxy/envoy:v1.31-latest
    container_name: envoy
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
      - "127.0.0.1:9901:9901"
    volumes:
      - ./out/envoy.generated.yaml:/etc/envoy/envoy.yaml:ro
      - /etc/letsencrypt/live/example.com:/etc/envoy/certs/example.com:ro
      - /etc/letsencrypt/archive/example.com:/etc/letsencrypt/archive/example.com:ro
    networks:
      - app-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9901/ready"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 10s
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"

  api:
    image: your-api:latest
    container_name: api
    restart: unless-stopped
    expose:
      - "3000"
    networks:
      - app-network
    environment:
      - NODE_ENV=production
      - DATABASE_URL=postgresql://db:5432/app
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  frontend:
    image: nginx:alpine
    container_name: frontend
    restart: unless-stopped
    expose:
      - "80"
    volumes:
      - ./frontend/dist:/usr/share/nginx/html:ro
    networks:
      - app-network

networks:
  app-network:
    driver: bridge
```

---

Happy deploying! 🚀

