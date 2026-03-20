# RedEye AI Engine

RedEye is a multi-service AI gateway platform with clear backend service ownership, a separate dashboard app, and shared infrastructure.

## Workspace Layout

```text
redeye-ai-engine/
|-- infra/                  # Docker init scripts and infra config
|-- redeye_auth/            # Authentication + onboarding service
|-- redeye_gateway/         # Core LLM gateway / admin APIs
|-- redeye_cache/           # Semantic cache service
|-- redeye_tracer/          # Trace ingestion / observability service
|-- redeye_compliance/      # Compliance and policy service
|-- redeye_dashboard/       # React dashboard + Tauri shell
|-- shared/                 # Cross-service reference modules / shared patterns
|-- fixtures/               # Sample payloads and local test data
|-- scripts/                # Setup and developer automation
|-- docs/                   # Guides, reports, architecture notes
|-- tools/                  # Local machine-specific helpers
|-- Cargo.toml              # Rust workspace manifest
|-- Cargo.lock              # Rust dependency lockfile
|-- docker-compose.yml      # Local container orchestration
|-- .env.example            # Environment template
|-- dev.bat                 # Windows local dev launcher
```

## Team Ownership Model

- Platform Team: `infra/`, `docker-compose.yml`, `scripts/`, `.env.example`
- Gateway Team: `redeye_gateway/`
- Identity Team: `redeye_auth/`
- Dashboard Team: `redeye_dashboard/`
- AI Runtime Team: `redeye_cache/`, `redeye_tracer/`, `redeye_compliance/`
- Architecture/Standards: `shared/`, `docs/`

## Daily Developer Entry Points

- Start local stack on Windows: `./dev.bat`
- Setup / bootstrap on Unix: `./scripts/setup.sh`
- Start infra only: `docker compose up -d postgres redis clickhouse`
- Dashboard app: `cd redeye_dashboard && npm run dev`
- Gateway service: `cargo run -p redeye_gateway`
- Auth service: `cargo run -p redeye_auth`
- Local CI verification: `powershell -ExecutionPolicy Bypass -File ./scripts/ci-verify.ps1`
- Local integration smoke test: `powershell -ExecutionPolicy Bypass -File ./scripts/integration-smoke.ps1`

## Documentation

- Quick start: `docs/guides/QUICKSTART.md`
- Production deployment: `docs/guides/PRODUCTION_DEPLOYMENT.md`
- Integration testing: `docs/guides/INTEGRATION_TESTING.md`
- Structure guide: `docs/PROJECT_STRUCTURE.md`
- Delivery reports: `docs/reports/`

## CI

- GitHub Actions workflow: `.github/workflows/ci.yml`
- Runs on push and pull request
- Covers:
  - `cargo check -p redeye_auth`
  - `cargo check -p redeye_gateway`
  - `cargo test -p redeye_gateway`
  - dashboard lint
  - dashboard build

## Working Rules

- Each service owns its own `src/`, dependencies, and runtime config.
- Shared infrastructure stays at the workspace root.
- Temporary payloads and manual test assets go into `fixtures/`, not root.
- Delivery notes and implementation reports go into `docs/reports/`, not root.
- Root should stay limited to workspace-level files only.
