# RedEye AI Engine — Quick Start Guide

**Get the system running in 5 minutes!**

---

## Prerequisites Check ✅

Before starting, ensure you have:

```bash
# Check Docker
docker --version      # v24.x or higher
docker compose version # v2.x or higher

# Check other tools
openssl version      # For generating secrets
jq --version        # For JSON parsing (optional)
git --version       # For version control
```

If any are missing, install them first:
- **Docker**: https://docs.docker.com/get-docker/
- **OpenSSL**: Pre-installed on macOS/Linux; included in Windows Git Bash
- **jq**: https://stedolan.github.io/jq/install/

---

## Step 1: Clone & Navigate

```bash
cd redeye-ai-engine
```

---

## Step 2: Generate Environment & Secrets

**Option A: Automatic (Recommended)**

```bash
chmod +x scripts/setup.sh
./scripts/setup.sh

# The script will:
# ✓ Generate secure secrets
# ✓ Create .env file
# ✓ Validate configuration
# ✓ Build Docker images
# ✓ Start services
# ✓ Wait for health checks
```

**Option B: Manual**

```bash
# Copy template
cp .env.example .env

# Generate secrets
export JWT_SECRET=$(openssl rand -base64 32)
export AES_MASTER_KEY=$(openssl rand -base64 32)
export POSTGRES_PASSWORD=$(openssl rand -base64 24)
export REDIS_PASSWORD=$(openssl rand -base64 24)

# Edit .env with your actual values
nano .env  # Set OPENAI_API_KEY to your real key
```

---

## Step 3: Start The Stack

```bash
# Start all services
docker compose up -d

# Wait 10 seconds for services to initialize
sleep 10

# Verify all are healthy
docker compose ps
```

**Expected Output:**
```
NAME                IMAGE                      STATUS
redeye_postgres     postgres:16-alpine         healthy
redeye_redis        redis/redis-stack-server   healthy
redeye_clickhouse   clickhouse-server:24.3     healthy
redeye_auth         redeye-ai-engine-auth      running
redeye_cache        redeye-ai-engine-cache     running
redeye_tracer       redeye-ai-engine-tracer    running
redeye_compliance   redeye-ai-engine-comp      running
redeye_gateway      redeye-ai-engine-gateway   running
```

---

## Step 4: Verify Services Are Working

### Check Gateway Health
```bash
curl http://localhost:8080/health
# Response: {"status": "ok"}
```

### Check All Services
```bash
curl http://localhost:8084/health  # Auth
curl http://localhost:8081/health  # Cache
curl http://localhost:8082/health  # Tracer
curl http://localhost:8083/health  # Compliance
```

---

## Step 5: Create a Test User

```bash
# Create user and get access token
TOKEN=$(curl -s -X POST http://localhost:8084/v1/auth/signup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@'$(date +%s)'@example.com",
    "password": "TestPassword123!",
    "tenant_name": "test-org"
  }' | jq -r '.access_token')

# Verify token was created
echo "Token: $TOKEN"
```

---

## Step 6: Test The Complete Flow

**Make an authenticated LLM request:**

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant"},
      {"role": "user", "content": "What is the capital of France?"}
    ],
    "temperature": 0.7,
    "max_tokens": 100
  }' | jq .
```

**Response includes:**
```json
{
  "id": "chatcmpl-8...",
  "object": "chat.completion",
  "created": 1711000000,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "The capital of France is Paris."
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 30,
    "completion_tokens": 5,
    "total_tokens": 35
  }
}
```

✅ **Success!** All 5 services are working together.

---

## Common Commands

### View Logs
```bash
# Gateway logs
docker compose logs -f redeye_gateway

# All services
docker compose logs -f

# Last 50 lines
docker compose logs --tail=50 redeye_gateway

# Filter by time
docker compose logs --since 2m redeye_gateway
```

### Restart Services
```bash
# Restart all
docker compose restart

# Restart specific service
docker compose restart redeye_gateway

# Rebuild and restart
docker compose up -d --build redeye_gateway
```

### Stop Everything
```bash
# Stop all services (keep data)
docker compose stop

# Start them again
docker compose start

# Stop and remove containers (keep data)
docker compose down

# Stop and delete everything (includes data)
docker compose down -v
```

### Check Service Status
```bash
# Detailed status
docker compose ps -a

# Show only running services
docker compose ps

# Check resource usage
docker compose stats
```

### Access Databases

**PostgreSQL:**
```bash
docker exec -it redeye_postgres psql -U RedEye -d RedEye

# Inside psql:
\dt                    # List tables
SELECT * FROM users;   # Query users
\q                     # Exit
```

**Redis:**
```bash
docker exec -it redeye_redis redis-cli -a $REDIS_PASSWORD
# Inside redis-cli:
KEYS *                 # List all keys
GET ratelimit:user:123 # Check rate limit
FLUSHDB                # Clear all (caution!)
```

**ClickHouse:**
```bash
docker exec -it redeye_clickhouse clickhouse-client \
  -u RedEye --password clickhouse_secret

# Inside clickhouse:
SHOW TABLES FROM RedEye_telemetry;
SELECT * FROM RedEye_telemetry.agent_traces LIMIT 10;
QUIT
```

---

## Testing Each Service Individually

### 1. Auth Service (8084) - User Management

```bash
# Create user
curl -X POST http://localhost:8084/v1/auth/signup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user-'$(date +%s)'@example.com",
    "password": "SecurePass123!",
    "tenant_name": "test-tenant"
  }'

# Expected: { "access_token": "...", "api_key": "..." }
```

### 2. Cache Service (8081) - Semantic Lookup

```bash
curl -X POST http://localhost:8081/v1/cache/lookup \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is machine learning?",
    "threshold": 0.95
  }'

# Expected: { "hit": false, "score": null }
```

### 3. Tracer Service (8082) - Telemetry Ingest

```bash
curl -X POST http://localhost:8082/v1/traces/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "correlation_id": "test-trace-123",
    "session_id": "sess-abc",
    "status_code": 200,
    "duration_ms": 150
  }'

# Expected: { "status": "success" }
```

### 4. Compliance Service (8083) - PII Redaction

```bash
curl -X POST http://localhost:8083/api/v1/compliance/redact \
  -H "Content-Type: application/json" \
  -d '{
    "text": "My email is john@example.com and SSN is 123-45-6789"
  }'

# Expected: Text with PII replaced by tokens
```

---

## Troubleshooting

### Services Won't Start
```bash
# Check logs
docker compose logs

# Verify environment variables
cat .env | grep -E "^(JWT_SECRET|AES_MASTER_KEY|POSTGRES_PASSWORD)"

# Ensure .env exists
ls -la .env

# Rebuild and restart
docker compose down -v
docker compose up -d
```

### Port Conflicts
```bash
# Check what's using port 8080
lsof -i :8080  # macOS/Linux
netstat -ano | findstr :8080  # Windows

# Use different ports in docker-compose.yml
ports:
  - "9080:8080"  # Use 9080 instead of 8080
```

### Database Connection Failed
```bash
# Check PostgreSQL is healthy
docker compose ps redeye_postgres

# Check connection string
docker exec redeye_postgres pg_isready -h localhost -U RedEye

# View PostgreSQL logs
docker compose logs redeye_postgres
```

### Memory/Resource Issues
```bash
# Check Docker resource limits
docker stats

# Increase Docker memory if low:
# Settings → Resources → Memory: 4GB+ recommended

# Clear Docker cache
docker system prune -a
```

---

## Performance Tips

### 1. Enable Caching
Edit `.env`:
```bash
CACHE_TTL=3600  # Cache responses for 1 hour
ENABLE_SEMANTIC_CACHE=true
```

### 2. Increase Rate Limits (Development Only)
Edit `.env`:
```bash
RATE_LIMIT_MAX_REQUESTS=1000
RATE_LIMIT_WINDOW_SECS=60
```

### 3. Monitor Performance
```bash
# Watch services in real-time
watch -n 1 'docker compose ps'

# Monitor logs for errors
docker compose logs -f | grep -i error
```

---

## Security Reminders

⚠️ **NEVER in Production:**
- ❌ Commit `.env` to Git
- ❌ Share API keys in code
- ❌ Use development secrets in production
- ❌ Run with `ENVIRONMENT=development` on production

✅ **DO in Production:**
- Use AWS Secrets Manager or Vault
- Enable TLS/SSL on all connections
- Set strong passwords (32+ bytes)
- Use separate databases per environment
- Enable audit logging
- Monitor for suspicious activity

---

## What's Running?

| Service | Port | Purpose |
|---------|------|---------|
| **Gateway** | 8080 | Main API - LLM proxy + orchestration |
| **Auth** | 8084 | User/API key management |
| **Cache** | 8081 | Semantic embedding lookup |
| **Tracer** | 8082 | Telemetry & audit logs |
| **Compliance** | 8083 | PII redaction & geo-routing |
| **PostgreSQL** | 5433 | User & tenant data |
| **Redis** | 6379 | Rate limiting & cache |
| **ClickHouse** | 8123 | Audit logging & analytics |

---

## Next Steps

1. **For Development:**
   - Read [docs/reports/OPTIMIZATION_SUMMARY.md](docs/reports/OPTIMIZATION_SUMMARY.md)
   - Check service logs: `docker compose logs -f redeye_gateway`
   - Modify code and rebuild: `docker compose up -d --build`

2. **For Production:**
   - Read [docs/guides/PRODUCTION_DEPLOYMENT.md](docs/guides/PRODUCTION_DEPLOYMENT.md)
   - Set up Kubernetes with Helm charts
   - Configure AWS Secrets Manager
   - Enable Jaeger distributed tracing

3. **For Testing:**
   - Run integration tests: `./scripts/integration-tests.sh`
   - Load test with k6: `k6 run tests/load-testing.js`
   - Check coverage: `cargo tarpaulin`

---

## Getting Help

- **Logs:** `docker compose logs -f <service>`
- **Health Check:** `curl http://localhost:8080/health`
- **Service Status:** `docker compose ps`
- **Full Guide:** See [docs/guides/PRODUCTION_DEPLOYMENT.md](docs/guides/PRODUCTION_DEPLOYMENT.md)
- **Optimization Details:** See [docs/reports/OPTIMIZATION_SUMMARY.md](docs/reports/OPTIMIZATION_SUMMARY.md)

---

## Summary

✅ You now have a **production-grade** microservices platform with:
- 5 integrated services running in Docker
- Professional error handling
- Secure configuration
- Full observability
- Complete documentation

**Happy coding! 🚀**

All services running step-by-step:
1. ✅ auth — User management
2. ✅ cache — Semantic caching  
3. ✅ compliance — PII redaction
4. ✅ gateway — LLM proxy
5. ✅ tracer — Telemetry

