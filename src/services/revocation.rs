use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Clone)]
pub struct RevocationService {
    pool: PgPool,
}

impl RevocationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn revoke_credential(
        &self,
        tenant_id: Uuid,
        credential_id: &str,
        reason: Option<String>,
        revoked_by: Option<String>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        let updated = sqlx::query(
            "UPDATE issued_credentials SET status = 'revoked', revoked_at = NOW(), revocation_reason = $1
             WHERE tenant_id = $2 AND credential_id = $3 AND status <> 'revoked'",
        )
        .bind(reason.clone())
        .bind(tenant_id)
        .bind(credential_id)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(AppError::NotFound(format!(
                "active credential not found: {credential_id}"
            )));
        }

        sqlx::query(
            "INSERT INTO credential_revocations (id, tenant_id, credential_id, reason, revoked_by, revoked_at)
             VALUES ($1, $2, $3, $4, $5, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(credential_id)
        .bind(reason)
        .bind(revoked_by)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}
