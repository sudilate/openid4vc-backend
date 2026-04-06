use std::sync::Arc;

use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::auth::authorization::AuthorizationService;
use crate::config::Settings;
use crate::services::did_resolver::DidResolverRegistry;
use crate::services::key_management::KeyManagementService;
use crate::services::oid4vci_runtime::Oid4vciRuntimeRegistry;
use crate::services::tenant_db::TenantDatabasePool;

#[derive(Clone)]
pub struct AppState {
    pub settings: Settings,
    pub admin_db: PgPool,
    pub redis: ConnectionManager,
    pub tenant_pools: Arc<TenantDatabasePool>,
    pub oid4vci_registry: Arc<Oid4vciRuntimeRegistry>,
    pub key_management: Arc<KeyManagementService>,
    pub did_registry: Arc<DidResolverRegistry>,
    pub authorization: Arc<AuthorizationService>,
}
