# OpenID4VC Backend

Production-oriented Rust backend for OpenID4VC (OpenID for Verifiable Credentials) with multi-tenant support.

## Overview

This service provides a complete OpenID4VC implementation including:
- **OID4VCI** (OpenID for Verifiable Credential Issuance) protocol support
- **OID4VP** (OpenID for Verifiable Presentations) for verification
- Multi-tenant architecture with isolated tenant databases
- Role-based access control (RBAC) with Oso policies
- Key management (Vault or file-based)
- Credential revocation with status list
- Audit logging and metrics
- Webhook delivery with HMAC signatures

## Architecture

```
┌─────────────────┐     ┌─────────────┐     ┌─────────────────┐
│   API Clients   │────▶│ Axum Server │────▶│  Auth Middleware│
└─────────────────┘     └──────┬──────┘     └─────────────────┘
                               │
          ┌────────────────────┼────────────────────┐
          ▼                    ▼                    ▼
   ┌──────────────┐   ┌──────────────┐   ┌──────────────────┐
   │ Admin DB     │   │ Tenant DBs   │   │ Redis            │
   │ (PostgreSQL) │   │ (PostgreSQL) │   │ (Session/Cache)  │
   └──────────────┘   └──────────────┘   └──────────────────┘
```

## Prerequisites

- Rust 1.75+ with Cargo
- PostgreSQL 15+
- Redis 7+
- Docker & Docker Compose (for local development)

## Quick Start

### 1. Start Infrastructure

```bash
docker compose up -d postgres redis vault
```

### 2. Configure Environment

```bash
cp .env.example .env
# Edit .env with your settings
```

### 3. Run Database Migrations

```bash
# Admin database
psql "postgres://postgres:postgres@localhost:5432/openid4vc_admin" -f migrations/001_admin_schema.sql
psql "postgres://postgres:postgres@localhost:5432/openid4vc_admin" -f migrations/003_admin_keys.sql
psql "postgres://postgres:postgres@localhost:5432/openid4vc_admin" -f migrations/005_admin_auth_audit.sql

# Create tenant database
psql "postgres://postgres:postgres@localhost:5432/postgres" -c "CREATE DATABASE tenant_acme;"

# Tenant schema
psql "postgres://postgres:postgres@localhost:5432/tenant_acme" -f migrations/002_tenant_schema.sql
psql "postgres://postgres:postgres@localhost:5432/tenant_acme" -f migrations/004_tenant_credential_lifecycle.sql
```

### 4. Run the Service

```bash
cargo run
```

### 5. Verify Installation

```bash
curl http://localhost:8080/health
curl http://localhost:8080/ready
curl http://localhost:8080/metrics
```

## Configuration

Configuration is loaded from `config/base.toml` and can be overridden via environment variables with the prefix `OID4VC_BACKEND__`.

Example:
```bash
OID4VC_BACKEND__SERVER__PORT=9090 cargo run
```

See `.env.example` for all available options.

## API Endpoints

### Health & Metrics

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/ready` | Readiness probe |
| GET | `/metrics` | Prometheus metrics |

### Tenant Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/tenants` | Create new tenant |
| POST | `/api/v1/tenants/{tenant_id}/api-keys` | Create API key for tenant |

### Issuer APIs

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/issuer/credential-definitions` | Create credential definition |
| POST | `/api/v1/issuer/offers` | Create credential offer |
| POST | `/api/v1/issuer/issued/{credential_id}/revoke` | Revoke issued credential |

### Verifier APIs

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/verifier/requests` | Create presentation request |

### DID Resolution

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/did/resolve?did={did}` | Resolve DID to DID document |

### Key Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/keys/rotate` | Rotate tenant primary key |

### Audit

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/audit/stream` | Server-sent events audit log stream |

### OID4VCI Protocol Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/oid4vci/{tenant_slug}/.well-known/oauth-authorization-server` | OAuth AS metadata |
| GET | `/oid4vci/{tenant_slug}/.well-known/openid-credential-issuer` | Credential issuer metadata |
| GET | `/oid4vci/{tenant_slug}/credential_offer` | Get credential offer |
| POST | `/oid4vci/{tenant_slug}/par` | Pushed authorization request |
| GET | `/oid4vci/{tenant_slug}/authorize` | Authorization endpoint |
| POST | `/oid4vci/{tenant_slug}/token` | Token endpoint |
| POST | `/oid4vci/{tenant_slug}/credential` | Credential endpoint |
| POST | `/oid4vci/{tenant_slug}/notification` | Notification endpoint |

## Authentication

The service supports multiple authentication methods:

1. **JWT Bearer tokens** - Production authentication via `Authorization: Bearer <token>` header
2. **API Keys** - Service-to-service via `X-API-Key` header
3. **Development headers** - Local development via `x-tenant-id`, `x-role`, and `x-principal` headers

### RBAC Roles

- `SuperAdmin` - Full access across all tenants
- `TenantAdmin` - Manage tenant settings and credential definitions
- `IssuerManager` - Issue credentials and manage offers
- `Verifier` - Create and manage presentation requests
- `ReadOnly` - View-only access
- `ApiClient` - Webhook and API access only

## Development

### Build

```bash
cargo build
cargo build --release
```

### Test

```bash
cargo test
cargo test <test_function_name>
cargo test -- --nocapture
```

### Lint & Format

```bash
cargo clippy
cargo fmt
cargo fmt -- --check
```

## Deployment

### Docker

```bash
docker build -t openid4vc-backend .
docker run -p 8080:8080 --env-file .env openid4vc-backend
```

### Kubernetes

See `deploy/k8s/` for Kubernetes manifests including:
- Deployment with health checks
- Service exposure
- ConfigMap for configuration
- HorizontalPodAutoscaler for scaling

```bash
kubectl apply -f deploy/k8s/
```

## Project Status

This is phase-1 scaffolding. The following features are planned:
- Full OID4VCI protocol endpoint wiring
- OID4VP protocol implementation
- Complete Oso policy enforcement
- Advanced revocation flows (Status List 2021)
- DIDComm messaging support

## License

[License information to be added]
