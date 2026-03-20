@echo off
setlocal
title RedEye AI Engine - Master Launcher

echo.
echo ============================================================
echo    RedEye AI Gateway - Full Stack Development Mode
echo ============================================================
echo.

:: Development env for locally-run Rust services
if not defined POSTGRES_USER set POSTGRES_USER=RedEye
if not defined POSTGRES_PASSWORD set POSTGRES_PASSWORD=RedEye_secret
if not defined POSTGRES_DB set POSTGRES_DB=RedEye
if not defined REDIS_PASSWORD set REDIS_PASSWORD=redis_secret
if not defined CLICKHOUSE_USER set CLICKHOUSE_USER=RedEye
if not defined CLICKHOUSE_PASSWORD set CLICKHOUSE_PASSWORD=clickhouse_secret
if not defined CLICKHOUSE_DB set CLICKHOUSE_DB=RedEye_telemetry
if not defined DATABASE_URL set DATABASE_URL=postgres://%POSTGRES_USER%:%POSTGRES_PASSWORD%@localhost:5433/%POSTGRES_DB%
if not defined REDIS_URL set REDIS_URL=redis://:%REDIS_PASSWORD%@localhost:6379
if not defined CLICKHOUSE_URL set CLICKHOUSE_URL=http://%CLICKHOUSE_USER%:%CLICKHOUSE_PASSWORD%@localhost:8123
if not defined GATEWAY_PORT set GATEWAY_PORT=8080
if not defined JWT_SECRET set JWT_SECRET=0123456789abcdef0123456789abcdef
if not defined AES_MASTER_KEY set AES_MASTER_KEY=abcdef0123456789abcdef0123456789
if not defined OPENAI_API_KEY set OPENAI_API_KEY=dummy
if not defined RUST_LOG set RUST_LOG=info

:: 1. Infrastructure Bootup
echo [1/3] Starting Docker Infrastructure (Postgres, Redis, ClickHouse)...
docker compose stop redeye_gateway redeye_auth > nul 2>&1
docker compose up -d postgres redis clickhouse
echo.
echo Waiting 5 seconds for DB and Cache to be ready...
timeout /t 5 /nobreak > nul

:: 2. Start Rust Microservices
echo [2/3] Launching Rust Services in separate windows...

:: Auth Service (Port 8084)
echo Starting RedEye Auth...
start "RedEye-Auth" cmd /c "cargo run -p redeye_auth"

:: Gateway (Port 8080)
echo Starting RedEye Gateway...
start "RedEye-Gateway" cmd /c "cargo run -p redeye_gateway"

:: Cache, Compliance & Tracer
echo Starting Supporting Services (Cache, Compliance, Tracer)...
start "RedEye-Cache" cmd /c "cargo run -p redeye_cache"
start "RedEye-Compliance" cmd /c "cargo run -p redeye_compliance"
start "RedEye-Tracer" cmd /c "cargo run -p redeye_tracer"

:: 3. Frontend Dashboard
echo [3/3] Launching React Dashboard...
cd redeye_dashboard
start "RedEye-Dashboard" cmd /c "npm run dev"

echo.
echo ============================================================
echo    ALL CARGO FEATURES ARE RUNNING! 🚀
echo ============================================================
echo.
echo Dashboard: http://localhost:5173
echo Gateway: http://localhost:8080
echo Auth: http://localhost:8084
echo.

pause
