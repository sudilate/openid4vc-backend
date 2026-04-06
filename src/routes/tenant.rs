use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
    pub database_url: String,
}

#[derive(Debug, Serialize)]
pub struct TenantResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
}

pub async fn create_tenant(
    State(state): State<AppState>,
    Json(payload): Json<CreateTenantRequest>,
) -> Result<Json<TenantResponse>, AppError> {
    let tenant_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO tenants (id, name, slug, database_url, is_active, created_at) VALUES ($1, $2, $3, $4, true, NOW())",
    )
    .bind(tenant_id)
    .bind(&payload.name)
    .bind(&payload.slug)
    .bind(&payload.database_url)
    .execute(&state.admin_db)
    .await?;

    Ok(Json(TenantResponse {
        id: tenant_id,
        name: payload.name,
        slug: payload.slug,
    }))
}
