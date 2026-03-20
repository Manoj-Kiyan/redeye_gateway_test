#!/bin/bash
# =========================================================================
# RedEye AI Engine - Professional Setup Script
# =========================================================================
# This script sets up the environment with secure defaults and validates
# all requirements before starting the services.

set -e  # Exit on error

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}RedEye AI Engine - Setup${NC}"
echo -e "${BLUE}========================================${NC}\n"

# =========================================================================
# 1. Check Prerequisites
# =========================================================================

echo -e "${BLUE}[1/6] Checking prerequisites...${NC}\n"

check_command() {
    if ! command -v $1 &> /dev/null; then
        echo -e "${RED}✗ $2 is not installed${NC}"
        exit 1
    else
        echo -e "${GREEN}✓ $1 found${NC}"
    fi
}

check_command "docker" "Docker"
check_command "docker-compose" "Docker Compose"
check_command "openssl" "OpenSSL"
check_command "jq" "jq (JSON processor)"

# Check Docker daemon is running
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}✗ Docker daemon is not running${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Docker daemon is running${NC}\n"

# =========================================================================
# 2. Generate Secure Secrets
# =========================================================================

echo -e "${BLUE}[2/6] Generating secure secrets...${NC}\n"

if [ -f .env ]; then
    echo -e "${YELLOW}? .env already exists. Backup and regenerate? (y/n)${NC}"
    read -r response
    if [[ "$response" == "y" ]]; then
        cp .env .env.backup.$(date +%s)
        echo -e "${GREEN}✓ Backed up to .env.backup.$(date +%s)${NC}"
    else
        echo -e "${YELLOW}Using existing .env${NC}"
        # Load existing secrets for validation
        set +a
        source .env
        set -a
    fi
fi

# Generate secrets if not present
if [ -z "$JWT_SECRET" ]; then
    export JWT_SECRET=$(openssl rand -base64 32)
    echo -e "${GREEN}✓ Generated JWT_SECRET${NC}"
fi

if [ -z "$AES_MASTER_KEY" ]; then
    export AES_MASTER_KEY=$(openssl rand -base64 32)
    echo -e "${GREEN}✓ Generated AES_MASTER_KEY${NC}"
fi

if [ -z "$API_KEY_SALT" ]; then
    export API_KEY_SALT=$(openssl rand -base64 16)
    echo -e "${GREEN}✓ Generated API_KEY_SALT${NC}"
fi

if [ -z "$POSTGRES_PASSWORD" ]; then
    export POSTGRES_PASSWORD=$(openssl rand -base64 24)
    echo -e "${GREEN}✓ Generated POSTGRES_PASSWORD${NC}"
fi

if [ -z "$REDIS_PASSWORD" ]; then
    export REDIS_PASSWORD=$(openssl rand -base64 24)
    echo -e "${GREEN}✓ Generated REDIS_PASSWORD${NC}"
fi

if [ -z "$CLICKHOUSE_PASSWORD" ]; then
    export CLICKHOUSE_PASSWORD=$(openssl rand -base64 24)
    echo -e "${GREEN}✓ Generated CLICKHOUSE_PASSWORD${NC}"
fi

# =========================================================================
# 3. Create .env File
# =========================================================================

echo -e "\n${BLUE}[3/6] Creating .env file...${NC}\n"

cat > .env << EOF
# ========================================================================
# RedEye AI Engine - Generated Configuration
# ========================================================================
# Generated: $(date)
# DO NOT COMMIT THIS FILE TO VERSION CONTROL!

# ── POSTGRES CREDENTIALS ───────────────────────────────────────────────
POSTGRES_USER=RedEye
POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
POSTGRES_DB=RedEye

# ── REDIS CREDENTIALS ──────────────────────────────────────────────────
REDIS_PASSWORD=${REDIS_PASSWORD}

# ── CLICKHOUSE CREDENTIALS ─────────────────────────────────────────────
CLICKHOUSE_USER=RedEye
CLICKHOUSE_PASSWORD=${CLICKHOUSE_PASSWORD}
CLICKHOUSE_DB=RedEye_telemetry

# ── SECURITY KEYS ──────────────────────────────────────────────────────
JWT_SECRET=${JWT_SECRET}
AES_MASTER_KEY=${AES_MASTER_KEY}
API_KEY_SALT=${API_KEY_SALT}

# ── OPENAI CONFIGURATION ──────────────────────────────────────────────
# REQUIRED: Add your actual OpenAI API key
OPENAI_API_KEY=sk-your_actual_openai_api_key_here

# ── ENVIRONMENT ────────────────────────────────────────────────────────
ENVIRONMENT=development
RUST_LOG=info

# ── FEATURE FLAGS ──────────────────────────────────────────────────────
ENABLE_RATE_LIMITING=true
ENABLE_SEMANTIC_CACHE=true
ENABLE_COMPLIANCE_ENGINE=true
ENABLE_AUDIT_LOGGING=true
EOF

echo -e "${GREEN}✓ Created .env file${NC}"
echo -e "${YELLOW}⚠ IMPORTANT: Edit .env and set OPENAI_API_KEY before starting!${NC}\n"

# =========================================================================
# 4. Validate Configuration
# =========================================================================

echo -e "${BLUE}[4/6] Validating configuration...${NC}\n"

# Load environment
set +a
source .env
set -a

# Validate secret lengths
if [ ${#JWT_SECRET} -lt 32 ]; then
    echo -e "${RED}✗ JWT_SECRET must be at least 32 bytes${NC}"
    exit 1
fi
echo -e "${GREEN}✓ JWT_SECRET length: ${#JWT_SECRET} bytes${NC}"

if [ ${#AES_MASTER_KEY} -ne 32 ]; then
    echo -e "${RED}✗ AES_MASTER_KEY must be exactly 32 bytes${NC}"
    exit 1
fi
echo -e "${GREEN}✓ AES_MASTER_KEY length: ${#AES_MASTER_KEY} bytes${NC}"

if [ "$OPENAI_API_KEY" == "sk-your_actual_openai_api_key_here" ]; then
    echo -e "${YELLOW}⚠ OPENAI_API_KEY is still a placeholder${NC}"
    echo -e "${YELLOW}  Edit .env and set your actual key before proceeding${NC}\n"
fi

# =========================================================================
# 5. Build Docker Images
# =========================================================================

echo -e "${BLUE}[5/6] Building Docker images...${NC}\n"

docker compose build --quiet

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Docker images built successfully${NC}\n"
else
    echo -e "${RED}✗ Docker build failed${NC}"
    exit 1
fi

# =========================================================================
# 6. Start Services
# =========================================================================

echo -e "${BLUE}[6/6] Starting services...${NC}\n"

docker compose up -d

echo -e "${GREEN}✓ Services started in detached mode${NC}\n"

# Wait for services to be healthy
echo -e "${BLUE}Waiting for services to become healthy (max 60s)...${NC}\n"

MAX_ATTEMPTS=30
ATTEMPT=0

while [ $ATTEMPT -lt $MAX_ATTEMPTS ]; do
    HEALTHY=$(docker compose ps --format "{{.Status}}" | grep -c "healthy" || true)
    TOTAL=$(docker compose ps --format "{{.Status}}" | wc -l)
    
    if [ "$HEALTHY" -eq "$TOTAL" ] 2>/dev/null; then
        echo -e "${GREEN}✓ All services are healthy${NC}"
        break
    fi
    
    echo "  [$((ATTEMPT+1))/$MAX_ATTEMPTS] Services healthy: $HEALTHY/$TOTAL"
    ATTEMPT=$((ATTEMPT+1))
    sleep 2
done

# =========================================================================
# Summary & Next Steps
# =========================================================================

echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN}Setup Complete! ✅${NC}"
echo -e "${GREEN}========================================${NC}\n"

echo -e "${BLUE}Service Endpoints:${NC}"
echo "  Gateway (LLM Proxy):   http://localhost:8080"
echo "  Auth Service:           http://localhost:8084"
echo "  Cache Service:          http://localhost:8081"
echo "  Tracer Service:         http://localhost:8082"
echo "  Compliance Service:     http://localhost:8083"
echo "  PostgreSQL:             localhost:5433"
echo "  Redis:                  localhost:6379"
echo "  ClickHouse:             localhost:8123"
echo ""

echo -e "${BLUE}Next Steps:${NC}"
echo "  1. Verify services are running:"
echo "     docker compose ps"
echo ""
echo "  2. Test gateway health:"
echo "     curl http://localhost:8080/health"
echo ""
echo "  3. Create a test user:"
echo "     ./scripts/create-test-user.sh"
echo ""
echo "  4. Check logs:"
echo "     docker compose logs -f redeye_gateway"
echo ""
echo "  5. Run integration tests:"
echo "     ./scripts/integration-tests.sh"
echo ""

echo -e "${YELLOW}Documentation:${NC}"
echo "  - Production Deployment: docs/guides/PRODUCTION_DEPLOYMENT.md"
echo "  - Architecture Guide:    README.md"
echo ""

echo -e "${GREEN}Happy coding! 🚀${NC}\n"

# Export variables for next commands
export COMPOSE_PROJECT_NAME=redeye-ai-engine

