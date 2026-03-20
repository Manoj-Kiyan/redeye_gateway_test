-- Add new columns to existing tenants table (which might be created by init.sql)
ALTER TABLE tenants
ADD COLUMN IF NOT EXISTS encrypted_openai_key BYTEA,
ADD COLUMN IF NOT EXISTS redeye_api_key TEXT UNIQUE,
ADD COLUMN IF NOT EXISTS onboarding_status BOOLEAN NOT NULL DEFAULT FALSE;

-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Provider credentials per tenant
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

-- Index tenant_id for fast lookup
CREATE INDEX IF NOT EXISTS idx_users_tenant_id ON users(tenant_id);
CREATE INDEX IF NOT EXISTS idx_provider_credentials_tenant_id ON provider_credentials(tenant_id);
