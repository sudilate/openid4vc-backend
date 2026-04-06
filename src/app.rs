use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use axum::extract::State;
use axum::middleware;
use axum::response::Response;
use axum::{http::Request, middleware::Next};
use chrono::Utc;
use redis::Client as RedisClient;
use redis::aio::ConnectionManager;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::auth::authorization::AuthorizationService;
use crate::config::Settings;
use crate::routes;
use crate::services::audit::{AuditEvent, AuditService};
use crate::services::did_resolver::DidResolverRegistry;
use crate::services::key_management::{KeyBackend, KeyManagementService};
use crate::services::metrics;
use crate::services::oid4vci_runtime::Oid4vciRuntimeRegistry;
use crate::services::tenant_db::TenantDatabasePool;
use crate::state::AppState;

pub async fn build_app(settings: Settings) -> anyhow::Result<axum::Router> {
    let admin_db = PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .connect(&settings.database.admin_url)
        .await
        .context("failed to connect to admin database")?;

    let redis_client = RedisClient::open(settings.redis.url.clone())
        .context("failed to initialize redis client")?;
    let redis = ConnectionManager::new(redis_client)
        .await
        .context("failed to initialize redis connection manager")?;

    let tenant_pools = Arc::new(TenantDatabasePool::new(admin_db.clone()));

    let key_backend = match settings.key_management.backend.as_str() {
        "vault" => KeyBackend::Vault {
            addr: settings
                .key_management
                .vault_addr
                .clone()
                .unwrap_or_else(|| "http://localhost:8200".to_string()),
            token: settings
                .key_management
                .vault_token
                .clone()
                .unwrap_or_else(|| "dev-token".to_string()),
        },
        _ => KeyBackend::File {
            base_path: settings.key_management.file_base_path.clone().into(),
        },
    };
    let key_management = Arc::new(KeyManagementService::new(admin_db.clone(), key_backend));

    let base_url = format!("http://{}:{}", settings.server.host, settings.server.port);
    let oid4vci_registry = Arc::new(Oid4vciRuntimeRegistry::new(
        admin_db.clone(),
        base_url,
        key_management.clone(),
    ));
    let did_registry = Arc::new(DidResolverRegistry::with_defaults());
    let audit = Arc::new(AuditService::new(admin_db.clone()));
    let authorization = Arc::new(AuthorizationService::new()?);

    metrics::init_metrics();

    let state = AppState {
        settings,
        admin_db,
        redis,
        tenant_pools,
        oid4vci_registry,
        key_management,
        did_registry,
        audit,
        authorization,
    };

    let middleware_state = state.clone();

    Ok(routes::router(state)
        .layer(middleware::from_fn_with_state(
            middleware_state,
            audit_and_metrics_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        ))
}

async fn audit_and_metrics_middleware(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let started = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let tenant_id = req
        .headers()
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| uuid::Uuid::parse_str(v).ok());
    let principal = req
        .headers()
        .get("x-principal")
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);

    let response = next.run(req).await;
    let status = response.status().as_u16();
    let elapsed = started.elapsed();

    metrics::HTTP_REQUESTS_TOTAL
        .with_label_values(&[&method, &path, &status.to_string()])
        .inc();
    metrics::HTTP_REQUEST_DURATION_SECONDS
        .with_label_values(&[&method, &path])
        .observe(elapsed.as_secs_f64());

    let audit_event = AuditEvent {
        id: uuid::Uuid::new_v4(),
        tenant_id,
        principal,
        method,
        path,
        status_code: status,
        latency_ms: elapsed.as_millis(),
        created_at: Utc::now(),
    };

    state.audit.emit(audit_event).await;
    response
}
