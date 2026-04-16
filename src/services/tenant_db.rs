use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::error::AppError;

#[derive(Clone)]
pub struct TenantDatabasePool {
    inner: Arc<RwLock<HashMap<Uuid, PgPool>>>,
    admin_pool: PgPool,
}

impl TenantDatabasePool {
    pub fn new(admin_pool: PgPool) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            admin_pool,
        }
    }

    pub async fn pool_for_tenant(&self, tenant_id: Uuid) -> Result<PgPool, AppError> {
        if let Some(existing) = self.inner.read().await.get(&tenant_id) {
            return Ok(existing.clone());
        }

        let db_url = sqlx::query_scalar::<_, String>(
            "SELECT database_url FROM tenants WHERE id = $1 AND is_active = true",
        )
        .bind(tenant_id)
        .fetch_one(&self.admin_pool)
        .await?;

        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&db_url)
            .await?;

        let _ = sqlx::query_scalar::<_, String>("SELECT set_config('app.current_tenant_id', $1, false)")
            .bind(tenant_id.to_string())
            .fetch_one(&pool)
            .await?;

        self.inner.write().await.insert(tenant_id, pool.clone());
        Ok(pool)
    }
}
