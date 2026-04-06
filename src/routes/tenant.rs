use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::auth::api_key::hash_api_key;
use crate::auth::authorization::Role;
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

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub role: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub role: String,
    pub api_key: String,
}

pub async fn create_tenant(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<CreateTenantRequest>,
) -> Result<Json<TenantResponse>, AppError> {
    if auth.role != Role::SuperAdmin {
        return Err(AppError::Forbidden(
            "only super_admin can create tenant".to_string(),
        ));
    }

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

pub async fn create_api_key(
    Path(tenant_id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<CreateApiKeyRequest>,
) -> Result<Json<CreateApiKeyResponse>, AppError> {
    if auth.role != Role::SuperAdmin && auth.tenant_id != tenant_id {
        return Err(AppError::Forbidden(
            "cross-tenant api key creation is forbidden".to_string(),
        ));
    }

    let key_id = Uuid::new_v4();
    let raw_api_key = format!("ok_{}", Uuid::new_v4());
    let key_hash = hash_api_key(&raw_api_key)?;
    let role = payload.role.unwrap_or_else(|| "api_client".to_string());

    sqlx::query(
        "INSERT INTO api_keys (id, tenant_id, name, key_hash, role, is_active, created_at)
         VALUES ($1, $2, $3, $4, $5, true, NOW())",
    )
    .bind(key_id)
    .bind(tenant_id)
    .bind(&payload.name)
    .bind(&key_hash)
    .bind(&role)
    .execute(&state.admin_db)
    .await?;

    Ok(Json(CreateApiKeyResponse {
        id: key_id,
        tenant_id,
        name: payload.name,
        role,
        api_key: raw_api_key,
    }))
}
