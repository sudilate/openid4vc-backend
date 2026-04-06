CREATE TABLE IF NOT EXISTS credential_definitions (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    name TEXT NOT NULL,
    format TEXT NOT NULL,
    schema JSONB NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS issuance_sessions (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    credential_definition_id UUID NOT NULL,
    pre_authorized_code TEXT,
    flow_type TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS verification_sessions (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    nonce TEXT NOT NULL,
    dcql_query JSONB NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);

ALTER TABLE credential_definitions ENABLE ROW LEVEL SECURITY;
ALTER TABLE issuance_sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE verification_sessions ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS tenant_isolation_credential_definitions ON credential_definitions;
CREATE POLICY tenant_isolation_credential_definitions ON credential_definitions
USING (tenant_id = current_setting('app.current_tenant_id')::UUID);

DROP POLICY IF EXISTS tenant_isolation_issuance_sessions ON issuance_sessions;
CREATE POLICY tenant_isolation_issuance_sessions ON issuance_sessions
USING (tenant_id = current_setting('app.current_tenant_id')::UUID);

DROP POLICY IF EXISTS tenant_isolation_verification_sessions ON verification_sessions;
CREATE POLICY tenant_isolation_verification_sessions ON verification_sessions
USING (tenant_id = current_setting('app.current_tenant_id')::UUID);
