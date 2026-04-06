use std::sync::Arc;

use anyhow::Context;
use redis::Client as RedisClient;
use redis::aio::ConnectionManager;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::auth::authorization::AuthorizationService;
use crate::config::Settings;
use crate::routes;
use crate::services::did_resolver::DidResolverRegistry;
use crate::services::key_management::{KeyBackend, KeyManagementService};
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
    let authorization = Arc::new(AuthorizationService::new()?);

    let state = AppState {
        settings,
        admin_db,
        redis,
        tenant_pools,
        oid4vci_registry,
        key_management,
        did_registry,
        authorization,
    };

    Ok(routes::router(state)
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        ))
}
