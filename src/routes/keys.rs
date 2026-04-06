use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::auth::AuthContext;
use crate::auth::authorization::{Action, Resource};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct RotateKeyResponse {
    pub tenant_id: uuid::Uuid,
    pub new_seed_hex: String,
}

pub async fn rotate_primary_key(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<RotateKeyResponse>, AppError> {
    if !state
        .authorization
        .is_allowed(auth.role, Resource::Tenant, Action::Update)
    {
        return Err(AppError::Forbidden(
            "role is not allowed to rotate tenant keys".to_string(),
        ));
    }

    let new_seed_hex = state
        .key_management
        .rotate_primary_key(auth.tenant_id)
        .await?;

    Ok(Json(RotateKeyResponse {
        tenant_id: auth.tenant_id,
        new_seed_hex,
    }))
}
