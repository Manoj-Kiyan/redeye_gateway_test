-- ==============================================================================
-- RedEye Policy Engine - PostgreSQL Schema
-- Runs automatically on first container boot via docker-entrypoint-initdb.d
-- ==============================================================================

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE IF NOT EXISTS tenants (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL UNIQUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    encrypted_openai_key BYTEA,
    redeye_api_key TEXT UNIQUE,
    onboarding_status BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS api_keys (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    key_hash    TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ,
    is_active   BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS rate_limit_policies (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    requests_per_min INTEGER NOT NULL DEFAULT 60,
    tokens_per_day   BIGINT  NOT NULL DEFAULT 1000000,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS llm_routes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider    TEXT NOT NULL CHECK (provider IN ('openai', 'anthropic', 'gemini')),
    model       TEXT NOT NULL,
    is_default  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS provider_credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('openai', 'anthropic', 'gemini')),
    encrypted_api_key BYTEA NOT NULL,
    is_primary BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, provider)
);

CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS admin_audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    actor_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    service TEXT NOT NULL CHECK (service IN ('auth', 'gateway')),
    action TEXT NOT NULL,
    target_type TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO tenants (id, name) VALUES
    ('00000000-0000-0000-0000-000000000001', 'acme-corp'),
    ('00000000-0000-0000-0000-000000000002', 'globex-inc')
ON CONFLICT DO NOTHING;

INSERT INTO rate_limit_policies (tenant_id, requests_per_min, tokens_per_day) VALUES
    ('00000000-0000-0000-0000-000000000001', 120, 5000000),
    ('00000000-0000-0000-0000-000000000002', 60,  1000000)
ON CONFLICT DO NOTHING;

INSERT INTO llm_routes (tenant_id, provider, model, is_default) VALUES
    ('00000000-0000-0000-0000-000000000001', 'openai',    'gpt-4o',                    TRUE),
    ('00000000-0000-0000-0000-000000000002', 'anthropic', 'claude-sonnet-4-20250514', TRUE)
ON CONFLICT DO NOTHING;
