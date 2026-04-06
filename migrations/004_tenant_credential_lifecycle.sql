CREATE TABLE IF NOT EXISTS issued_credentials (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    credential_id TEXT NOT NULL,
    credential_configuration_id TEXT NOT NULL,
    subject_did TEXT NOT NULL,
    issuer_did TEXT NOT NULL,
    credential_raw TEXT NOT NULL,
    claims JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    revocation_reason TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_issued_credentials_credential_id
    ON issued_credentials (credential_id);

CREATE TABLE IF NOT EXISTS credential_revocations (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    credential_id TEXT NOT NULL,
    reason TEXT,
    revoked_by TEXT,
    revoked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE issued_credentials ENABLE ROW LEVEL SECURITY;
ALTER TABLE credential_revocations ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS tenant_isolation_issued_credentials ON issued_credentials;
CREATE POLICY tenant_isolation_issued_credentials ON issued_credentials
USING (tenant_id = current_setting('app.current_tenant_id')::UUID);

DROP POLICY IF EXISTS tenant_isolation_credential_revocations ON credential_revocations;
CREATE POLICY tenant_isolation_credential_revocations ON credential_revocations
USING (tenant_id = current_setting('app.current_tenant_id')::UUID);
