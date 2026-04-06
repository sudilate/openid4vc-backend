# AGENTS.md - OpenID4VC Backend

Production-oriented Rust backend for OpenID4VC with multi-tenant support. Built with Axum, PostgreSQL, Redis, and local OpenID4VC crates.

## Build, Lint, and Test Commands

```bash
cargo build
cargo build --release
cargo run
cargo test
cargo test --test <test_name>
cargo test <test_function_name>
cargo test -- --nocapture
cargo clippy
cargo clippy -- -D warnings
cargo fmt
cargo fmt -- --check
```

## Database Migrations

```bash
psql "postgres://postgres:postgres@localhost:5432/openid4vc_admin" -f migrations/001_admin_schema.sql
psql "postgres://postgres:postgres@localhost:5432/openid4vc_admin" -f migrations/003_admin_keys.sql
psql "postgres://postgres:postgres@localhost:5432/openid4vc_admin" -f migrations/005_admin_auth_audit.sql
psql "postgres://postgres:postgres@localhost:5432/tenant_<name>" -f migrations/002_tenant_schema.sql
psql "postgres://postgres:postgres@localhost:5432/tenant_<name>" -f migrations/004_tenant_credential_lifecycle.sql
docker compose up -d postgres redis vault
```

## Code Style Guidelines

### Imports
Group imports: std, external crates, local modules. Separate with blank lines.

```rust
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::error::AppError;
use crate::state::AppState;
```

### Formatting
- Use `cargo fmt` for all formatting
- Max line length: 100 characters
- Indent with 4 spaces
- No trailing whitespace
- Single blank line between functions

### Naming Conventions
- Functions/variables: `snake_case`
- Types/structs/enums: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Modules/files: `snake_case`

### Struct Definitions
Use `#[derive(...)]` with order: Debug, Clone, Serialize/Deserialize, others.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
    pub database_url: String,
}
```

### Error Handling
Use `AppError` from `src/error.rs`:

```rust
pub async fn handler() -> Result<Json<Response>, AppError> {
    // ...
}
```

Error variants: `Unauthorized` (401), `Forbidden` (403), `BadRequest` (400), `NotFound` (404), `Internal` (500), `Anyhow` (500). Use `anyhow` for context: `.context("message")?`

### Route Handlers
Standard pattern:

```rust
pub async fn handler(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<RequestType>,
) -> Result<Json<ResponseType>, AppError> {
    if !state.authorization.is_allowed(auth.role, Resource::X, Action::Y) {
        return Err(AppError::Forbidden("message".to_string()));
    }
    Ok(Json(response))
}
```

### Services
Wrap in `Arc<T>`, store in `AppState`:

```rust
#[derive(Clone)]
pub struct MyService {
    pub admin_pool: PgPool,
}

impl MyService {
    pub fn new(admin_pool: PgPool) -> Self {
        Self { admin_pool }
    }
}
```

### Database Queries
Use `sqlx`:

```rust
sqlx::query("INSERT INTO table (col) VALUES ($1)")
    .bind(value)
    .execute(&pool)
    .await?;
```

For tenant databases: `let tenant_pool = state.tenant_pools.pool_for_tenant(auth.tenant_id).await?;`

### Authentication and Authorization
- Use `AuthContext` extractor for protected routes
- Check permissions: `state.authorization.is_allowed(role, resource, action)`
- Roles: `SuperAdmin`, `TenantAdmin`, `IssuerManager`, `Verifier`, `ReadOnly`, `ApiClient`
- Resources: `CredentialDefinition`, `IssuedCredential`, `IssuanceSession`, `VerificationSession`, `Webhook`, `Tenant`
- Actions: `Create`, `Read`, `Update`, `Delete`, `Issue`, `Verify`, `Revoke`

### Configuration
- Environment variables with prefix `OID4VC_BACKEND__`
- Use `Settings::from_env()` to load
- Configuration in `config/base.toml`
- See `.env.example` for all options

### Module Structure
```
src/
├── main.rs           # Entry point
├── lib.rs            # Module declarations
├── app.rs            # App builder and middleware
├── state.rs          # AppState definition
├── config.rs         # Configuration structs
├── error.rs          # AppError enum
├── auth/             # Authentication/authorization
├── routes/           # HTTP route handlers
├── services/         # Business logic services
├── idv/              # Identity verification plugins
└── tenant/           # Tenant-specific logic
```

### Comments
Do NOT add comments unless explicitly requested. Code should be self-documenting through clear naming.

### Async and Concurrency
- Use `async fn` for all route handlers and service methods
- Use `Arc<T>` for shared state
- Use `tokio::sync` primitives for synchronization
- Prefer `await?` pattern for error propagation

### Testing
- Place tests in same file with `#[cfg(test)]` module
- Use `tokio-test` for async tests
- Test structure: arrange, act, assert
- Mock external dependencies when possible

### Security
- Never log secrets, API keys, or JWT tokens
- Use `hash_api_key()` for storing API key hashes
- Validate all user input
- Use parameterized queries (sqlx does this automatically)
- Check tenant isolation in multi-tenant operations

### Dependencies
Key: `axum` (web), `sqlx` (database), `serde`/`serde_json` (serialization), `thiserror` (errors), `anyhow` (error handling), `tokio` (async), `uuid` (UUIDs), `chrono` (date/time), `tracing` (logging). Local: `oid4vc-manager`, `oid4vc-core`, `oid4vci`, `oid4vp`, `siopv2`.

### Common Patterns

**Creating a new route:**
1. Add handler in `src/routes/*.rs`
2. Add route to `src/routes/mod.rs` router
3. Create request/response DTOs
4. Add authorization check
5. Implement business logic

**Creating a new service:**
1. Create `src/services/<name>.rs`
2. Add struct with dependencies
3. Implement `new()` constructor
4. Add business methods
5. Export in `src/services/mod.rs`
6. Add to `AppState` in `src/state.rs`
7. Initialize in `src/app.rs`

**Adding a new API endpoint:**
1. Define route handler with proper extractors
2. Add authorization check
3. Validate input
4. Call service layer
5. Return response
6. Add route to router in `mod.rs`
