use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub db: PgPool,
}
