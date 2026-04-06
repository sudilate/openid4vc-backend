use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub principal: Option<String>,
    pub method: String,
    pub path: String,
    pub status_code: u16,
    pub latency_ms: u128,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct AuditService {
    pub admin_pool: PgPool,
    sender: Arc<broadcast::Sender<AuditEvent>>,
}

impl AuditService {
    pub fn new(admin_pool: PgPool) -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self {
            admin_pool,
            sender: Arc::new(sender),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AuditEvent> {
        self.sender.subscribe()
    }

    pub async fn emit(&self, event: AuditEvent) {
        let _ = self.sender.send(event.clone());

        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, principal, method, path, status_code, latency_ms, created_at, metadata)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(event.id)
        .bind(event.tenant_id)
        .bind(event.principal)
        .bind(event.method)
        .bind(event.path)
        .bind(i32::from(event.status_code))
        .bind(event.latency_ms as i64)
        .bind(event.created_at)
        .bind(serde_json::json!({}))
        .execute(&self.admin_pool)
        .await;
    }
}
