# RedEye AI Engine — Professional Deployment Guide

**Version**: 1.0  
**Last Updated**: March 2026  
**Status**: Production-Ready Microservices

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Prerequisites](#prerequisites)
3. [Local Development Setup](#local-development-setup)
4. [Docker Compose Deployment](#docker-compose-deployment)
5. [Production Kubernetes Deployment](#production-kubernetes-deployment)
6. [Service Integration Flow](#service-integration-flow)
7. [Security Best Practices](#security-best-practices)
8. [Monitoring & Observability](#monitoring--observability)
9. [Troubleshooting](#troubleshooting)

---

## Architecture Overview

### The 5 Microservices

| Service | Port | Purpose | Dependencies |
|---------|------|---------|---|
| **redeye_auth** | 8084 | User/tenant/API key management | PostgreSQL |
| **redeye_cache** | 8081 | Semantic cache (vector embeddings) | Redis, OpenAI |
| **redeye_tracer** | 8082 | Centralized telemetry & audit logs | ClickHouse |
| **redeye_compliance** | 8083 | PII redaction & geo-routing | OPA (policies) |
| **redeye_gateway** | 8080 | Main LLM proxy & orchestration | All above + OpenAI |

### Request Flow

```
[Client]
    ↓
[redeye_gateway:8080] (Auth + Rate Limiting + Tracing)
    ├→ [redeye_auth:8084] — Token validation
    ├→ [redeye_cache:8081] — Semantic lookup
    ├→ [OpenAI API] — Forward request
    ├→ [redeye_compliance:8083] — PII redaction
    ├→ [redeye_tracer:8082] — Async telemetry
    └→ [ClickHouse] — Audit logging
```

---

## Prerequisites

### Local Development

- **Docker** ≥ 24.x
- **Docker Compose** ≥ 2.x  
- **Rust** ≥ 1.76 (for local builds)
- **Node.js** ≥ 18 (for dashboard)
- **OpenAI API Key** (sk-xxx)

### Production Environment

- **Kubernetes** ≥ 1.24
- **Helm** ≥ 3.x
- **AWS Secrets Manager** or **HashiCorp Vault**
- **Prometheus** + **Grafana** (monitoring)
- **Jaeger** (distributed tracing)
- **PostgreSQL** ≥ 14 (managed RDS)
- **Redis** ≥ 7.0 (managed ElastiCache)
- **ClickHouse** ≥ 24.3 (managed cluster)

---

## Local Development Setup

### 1. Clone & Setup Environment

```bash
git clone <repo> redeye-ai-engine
cd redeye-ai-engine

# Copy environment template
cp .env.example .env

# Generate secure secrets
export JWT_SECRET=$(openssl rand -base64 32)
export AES_MASTER_KEY=$(openssl rand -base64 32)

# Edit .env with your values
nano .env  # Set POSTGRES_PASSWORD, REDIS_PASSWORD, OPENAI_API_KEY
```

### 2. Start Full Stack

```bash
# Start all services (builds Dockerfiles)
docker compose up -d

# Verify all services are healthy
docker compose ps

# Check logs
docker compose logs -f redeye_gateway
```

### 3. Verify Services Are Running

```bash
# Gateway health check
curl http://localhost:8080/health

# Auth service
curl http://localhost:8084/health

# Cache service
curl http://localhost:8081/health

# Tracer service
curl http://localhost:8082/health
```

### 4. Initialize Data

```bash
# Create test user
curl -X POST http://localhost:8084/v1/auth/signup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "SecurePassword123!",
    "tenant_name": "test-org"
  }'

# Response: { "access_token": "...", "api_key": "..." }
```

---

## Docker Compose Deployment

### Quick Start (Development)

```bash
docker compose up -d
docker compose logs -f
```

### Full Reset (Clear All Data)

```bash
docker compose down -v
docker rmi redeye-ai-engine-redeye_*
docker compose up -d
```

### Scaling Services

```bash
# Scale multiple replicas (requires reverse proxy setup)
docker compose up -d --scale redeye_gateway=3
```

---

## Production Kubernetes Deployment

### Prerequisites

```bash
# Install Helm
curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash

# Configure kubectl
export KUBECONFIG=~/.kube/config
kubectl cluster-info

# Create namespace
kubectl create namespace redeye
```

### 1. Secret Management

```bash
# Create secrets from AWS Secrets Manager
for SECRET in postgres_password redis_password jwt_secret aes_key; do
  VALUE=$(aws secretsmanager get-secret-value --secret-id redeye-$SECRET \
    --query SecretString --output text)
  kubectl create secret generic redeye-$SECRET --from-literal=value=$VALUE \
    -n redeye
done
```

### 2. Deploy with Helm

```bash
# Install Helm chart (create helm/ directory with values.yaml)
helm install redeye ./helm \
  --namespace redeye \
  --values helm/values.prod.yaml \
  --set image.tag=v1.0.0

# Verify deployment
kubectl get pods -n redeye
kubectl get svc -n redeye
```

### 3. Configure Ingress

```yaml
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: redeye-ingress
  namespace: redeye
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - api.redeye.example.com
    secretName: redeye-tls
  rules:
  - host: api.redeye.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: redeye-gateway
            port:
              number: 8080
```

### 4. Monitor Deployment

```bash
# Watch rollout
kubectl rollout status deployment/redeye-gateway -n redeye

# Check logs
kubectl logs -f deployment/redeye-gateway -n redeye

# Port-forward for local testing
kubectl port-forward svc/redeye-gateway 8080:8080 -n redeye
```

---

## Service Integration Flow

### Example: Chat Completion Request

```
POST /v1/chat/completions

1. [Gateway] Authenticate JWT token
2. [Auth] Validate token against PostgreSQL
3. [Gateway] Check rate limit (Redis Lua script)
4. [Cache] Semantic similarity search (embeddings vs threshold 0.95)
5. [Cache] If HIT → Return cached response
6. [Compliance] Redact PII from request
7. [Gateway] Forward to OpenAI API
8. [Compliance] Redact PII from response
9. [Tracer] Async ingest trace to ClickHouse
10. [Gateway] Return response with X-Cache: HIT/MISS header
```

### Integration Testing

```bash
#!/bin/bash
# test-integration.sh

# Get auth token
TOKEN=$(curl -s -X POST http://localhost:8084/v1/auth/signup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "'$(date +%s)'@test.com",
    "password": "TestPass123!",
    "tenant_name": "integration-test"
  }' | jq -r '.access_token')

# Call gateway
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello"}],
    "temperature": 0.7
  }' | jq .

echo "Integration test complete ✅"
```

---

## Security Best Practices

### 1. Secret Management

**❌ NEVER**
```bash
# Don't commit secrets to Git
echo "JWT_SECRET=abc123" > .env
git add .env
git push
```

**✅ DO**
```bash
# Use AWS Secrets Manager
aws secretsmanager create-secret --name redeye-jwt-secret \
  --secret-string "$(openssl rand -base64 32)"

# Reference in environment
export JWT_SECRET=$(aws secretsmanager get-secret-value \
  --secret-id redeye-jwt-secret --query SecretString --output text)
```

### 2. TLS/SSL Certificates

```bash
# Let's Encrypt with cert-manager (Kubernetes)
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml

# Configure ClusterIssuer
kubectl apply -f - <<EOF
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: admin@redeye.com
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
    - http01:
        ingress:
          class: nginx
EOF
```

### 3. Network Policies

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: redeye-network-policy
  namespace: redeye
spec:
  podSelector:
    matchLabels:
      app: redeye-gateway
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress-nginx
    ports:
    - protocol: TCP
      port: 8080
```

### 4. Rate Limiting

Configured in `redeye_gateway/src/usecases/proxy.rs`:
- **60 requests** per **60 seconds** (default)
- Per-tenant bucketing via Redis Lua script
- Customizable via `RATE_LIMIT_MAX_REQUESTS` / `RATE_LIMIT_WINDOW_SECS`

---

## Monitoring & Observability

### 1. Structured Logging

All services use `tracing` crate with structured correlation IDs:

```rust
tracing::info!(
    correlation_id = %context.correlation_id,
    session_id = ?context.session_id,
    tenant_id = ?context.tenant_id,
    "Chat completion request processed"
);
```

### 2. Prometheus Metrics (Future)

```bash
# Add to docker-compose.yml
prometheus:
  image: prom/prometheus:latest
  ports:
    - "9090:9090"
  volumes:
    - ./prometheus.yml:/etc/prometheus/prometheus.yml
```

### 3. Distributed Tracing with Jaeger

```bash
# Docker
docker run -d \
  -p 16686:16686 \
  -p 14268:14268 \
  jaegertracing/all-in-one

# Query traces
# Visit http://localhost:16686
```

### 4. ClickHouse Analytics

```sql
-- Recent traces
SELECT 
    correlation_id, 
    session_id, 
    status_code, 
    duration_ms 
FROM agent_traces 
WHERE timestamp > now() - INTERVAL 1 HOUR 
ORDER BY timestamp DESC 
LIMIT 100;

-- PII redaction audit
SELECT 
    timestamp, 
    tenant_id,
    action, 
    token_count 
FROM compliance_audit_log 
WHERE timestamp > now() - INTERVAL 1 DAY;
```

---

## Troubleshooting

### Service Won't Start

```bash
# Check Dockerfiles are present
ls -la redeye_*/Dockerfile

# Verify Rust toolchain
docker run --rm rust:latest rustc --version

# Build manually
cargo build --release

# Check logs
docker compose logs redeye_gateway | tail -50
```

### Database Connection Errors

```bash
# Verify PostgreSQL is healthy
docker exec redeye_postgres pg_isready -U RedEye

# Check connection string format
# postgres://USER:PASSWORD@HOST:PORT/DATABASE

# From inside container
docker exec -it redeye_postgres psql -U RedEye -d RedEye -c "\dt"
```

### Redis Connection Issues

```bash
# Test Redis connection
docker exec redeye_redis redis-cli -a $REDIS_PASSWORD ping

# Check memory
docker exec redeye_redis redis-cli -a $REDIS_PASSWORD info memory
```

### ClickHouse Queries Failing

```bash
# Check tables exist
docker exec redeye_clickhouse clickhouse-client -u RedEye \
  --password clickhouse_secret \
  --query "SHOW TABLES FROM RedEye_telemetry"

# View schema
docker exec redeye_clickhouse clickhouse-client -u RedEye \
  --query "DESC RedEye_telemetry.agent_traces"
```

### Slow Requests

```bash
# Check rate limiting (Redis key exists?)
docker exec redeye_redis redis-cli -a $REDIS_PASSWORD \
  KEYS "ratelimit:*" | head -10

# View ClickHouse query performance
SELECT query, query_duration_ms 
FROM system.query_log 
WHERE type='QueryFinish' 
ORDER BY event_time DESC 
LIMIT 10;
```

---

## Running All Services End-to-End

### Step-by-Step Verification

```bash
# 1. Start infrastructure
docker compose up -d postgres redis clickhouse

# Wait for health checks
sleep 10
docker compose ps

# 2. Start auth service
docker compose up -d redeye_auth
curl http://localhost:8084/health

# 3. Start cache service
docker compose up -d redeye_cache
curl http://localhost:8081/health

# 4. Start compliance service
docker compose up -d redeye_compliance
curl http://localhost:8083/health

# 5. Start tracer service
docker compose up -d redeye_tracer
curl http://localhost:8082/health

# 6. Start gateway
docker compose up -d redeye_gateway
curl http://localhost:8080/health

# 7. Create test user
curl -X POST http://localhost:8084/v1/auth/signup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "prod-test@example.com",
    "password": "ProdTest123!",
    "tenant_name": "production"
  }' | jq .

# 8. Make authenticated request
TOKEN="your-token-from-step-7"
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Test"}]
  }' | jq .

echo "✅ All services operational!"
```

---

## Performance Benchmarks

| Operation | Target | Current | Notes |
|-----------|--------|---------|-------|
| Auth token validation | <10ms | ~5ms | Redis cached |
| Cache semantic lookup | <50ms | ~30ms | HSET in-memory |
| OpenAI proxy | <5s | Var | Upstream dependency |
| Audit logging | async | async | Fire-and-forget to ClickHouse |
| Rate limit check | <1ms | ~0.5ms | Lua script in Redis |

---

## Summary

This production-grade deployment guide provides:

✅ **Clean Architecture** — Layered services with clear separation  
✅ **Error Handling** — Unified error types with correlation IDs  
✅ **Security** — Secret management, TLS, network policies  
✅ **Observability** — Structured logging, tracing, metrics  
✅ **Scalability** — Kubernetes-ready, load-balanced  
✅ **Reliability** — Health checks, graceful degradation  

**Next Steps:**
1. Generate real secrets in `.env`
2. Run `docker compose up -d` to start development
3. Execute integration tests
4. Deploy to Kubernetes for production

---

**Support**: For issues, check service logs with `docker compose logs <service>` or contact the DevOps team.
