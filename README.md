# openid4vc-backend

Production-oriented Rust backend scaffold for OpenID4VC with multi-tenant support.

## What is included

- Axum API service with health and readiness endpoints
- Multi-tenant admin database + per-tenant database routing
- Row-level security migration bootstrap
- Initial issuer and verifier API endpoints
- Auth context extraction (tenant + role headers)
- RBAC bootstrap and Oso policy file scaffold
- IDV plugin registry trait for external providers
- Webhook delivery service scaffold with HMAC signature
- Docker Compose stack (PostgreSQL, Redis, Vault, API)
- Kubernetes manifests for deployment and autoscaling

## Quick start

1. Start infra:

```bash
docker compose up -d postgres redis vault
```

2. Run migrations (admin database):

```bash
psql "postgres://postgres:postgres@localhost:5432/openid4vc_admin" -f migrations/001_admin_schema.sql
psql "postgres://postgres:postgres@localhost:5432/openid4vc_admin" -f migrations/003_admin_keys.sql
psql "postgres://postgres:postgres@localhost:5432/openid4vc_admin" -f migrations/005_admin_auth_audit.sql
```

3. Create a tenant database and run tenant schema migration:

```bash
createdb -h localhost -U postgres tenant_acme
psql "postgres://postgres:postgres@localhost:5432/tenant_acme" -f migrations/002_tenant_schema.sql
psql "postgres://postgres:postgres@localhost:5432/tenant_acme" -f migrations/004_tenant_credential_lifecycle.sql
```

4. Run service:

```bash
cargo run
```

5. Health check:

```bash
curl http://localhost:8080/health
```

## Current API bootstrap

- `GET /health`
- `GET /ready`
- `GET /metrics`
- `POST /api/v1/tenants`
- `POST /api/v1/tenants/{tenant_id}/api-keys`
- `GET /api/v1/audit/stream` (SSE)
- `GET /api/v1/did/resolve?did=did:web:example.com`
- `POST /api/v1/keys/rotate`
- `POST /api/v1/issuer/credential-definitions`
- `POST /api/v1/issuer/offers`
- `POST /api/v1/issuer/issued/{credential_id}/revoke`
- `POST /api/v1/verifier/requests`

### OID4VCI protocol endpoints

- `GET /oid4vci/{tenant_slug}/.well-known/oauth-authorization-server`
- `GET /oid4vci/{tenant_slug}/.well-known/openid-credential-issuer`
- `GET /oid4vci/{tenant_slug}/credential_offer`
- `POST /oid4vci/{tenant_slug}/par`
- `GET /oid4vci/{tenant_slug}/authorize`
- `POST /oid4vci/{tenant_slug}/token`
- `POST /oid4vci/{tenant_slug}/credential`
- `POST /oid4vci/{tenant_slug}/notification`

## Notes

- This is phase-1 scaffolding. OID4VC protocol endpoint wiring, Oso enforcement, API key/JWT validation, and revocation flows are next.
- For local dev, auth context is read from `x-tenant-id`, `x-role`, and `x-principal` headers.
- DID resolver registry ships with `did:key`, `did:web`, and `did:ion` plugins.
