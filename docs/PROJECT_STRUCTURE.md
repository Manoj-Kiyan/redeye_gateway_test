# Project Structure

## Goal

This workspace is organized for multiple teams working in parallel without stepping on each other.

## Top-Level Rules

- Keep only workspace-level files in the repository root.
- Keep service-specific code inside the owning service folder.
- Put docs in `docs/`, sample payloads in `fixtures/`, and automation in `scripts/`.
- Avoid adding one-off files directly to the root.

## Modules

### `redeye_gateway/`
- Main API gateway
- LLM proxy endpoints
- Admin metrics endpoints
- Integration point for cache, tracer, compliance, Redis, and Postgres

### `redeye_auth/`
- Signup / login / refresh
- Workspace onboarding
- API key encryption and JWT handling

### `redeye_dashboard/`
- React frontend
- Presentation, domain, and data layers separated under `src/`
- Tauri desktop shell under `src-tauri/`

### `redeye_cache/`
- Semantic cache responsibilities
- Cache hit/miss workflows
- Redis-backed caching logic

### `redeye_tracer/`
- Request trace capture
- Audit and observability concerns

### `redeye_compliance/`
- Policy and compliance checks
- Guardrail enforcement and validation

## Shared Workspace Folders

### `infra/`
- Postgres and ClickHouse bootstrap SQL
- Container config and infra defaults

### `scripts/`
- Setup scripts
- Local automation
- Future CI helpers and migration utilities

### `shared/`
- Shared patterns and reference implementations
- Common error/config approaches that may later become a crate

### `fixtures/`
- Manual API payloads
- Seed examples
- Local testing JSON files

### `docs/guides/`
- Human-readable runbooks and setup docs

### `docs/reports/`
- Delivery notes, implementation summaries, and internal reports

### `docs/notes/`
- Internal scratch notes and temporary planning docs

## Recommended Team Boundaries

- Team A: `redeye_gateway/`
- Team B: `redeye_auth/`
- Team C: `redeye_dashboard/`
- Team D: `redeye_cache/` + `redeye_tracer/`
- Team E: `redeye_compliance/`
- Platform Team: `infra/`, `scripts/`, `docker-compose.yml`, `.env.example`

## Root File Checklist

Allowed in root:
- `Cargo.toml`
- `Cargo.lock`
- `.gitignore`
- `.env.example`
- `docker-compose.yml`
- `README.md`
- `dev.bat`
- primary service folders
- workspace support folders (`docs/`, `scripts/`, `fixtures/`, `shared/`, `infra/`, `tools/`)

Avoid in root:
- ad-hoc markdown reports
- sample JSON payloads
- one-off shell scripts
- temporary task notes
- shared experimental Rust files
