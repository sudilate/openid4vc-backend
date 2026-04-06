CREATE TABLE IF NOT EXISTS tenant_issuer_keys (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    key_id TEXT NOT NULL,
    did TEXT NOT NULL,
    backend TEXT NOT NULL,
    algorithm TEXT NOT NULL,
    seed_hex TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    is_primary BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    rotated_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_tenant_primary_key
    ON tenant_issuer_keys (tenant_id)
    WHERE is_primary = TRUE AND status = 'active';
