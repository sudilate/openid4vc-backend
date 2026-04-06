use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::auth::authorization::{Action, Resource};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreatePresentationRequest {
    pub credential_types: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CreatePresentationResponse {
    pub verification_session_id: Uuid,
    pub request_url: String,
}

pub async fn create_presentation_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<CreatePresentationRequest>,
) -> Result<Json<CreatePresentationResponse>, AppError> {
    if !state
        .authorization
        .is_allowed(auth.role, Resource::VerificationSession, Action::Create)
    {
        return Err(AppError::Forbidden(
            "role is not allowed to create verification sessions".to_string(),
        ));
    }

    let verification_session_id = Uuid::new_v4();
    let nonce = Uuid::new_v4().to_string();
    let tenant_pool = state.tenant_pools.pool_for_tenant(auth.tenant_id).await?;

    let dcql_query = serde_json::json!({
        "credentials": payload.credential_types.into_iter().map(|credential_type| serde_json::json!({
            "id": format!("q-{}", Uuid::new_v4()),
            "format": "jwt_vc_json",
            "meta": {"vct_values": [credential_type]}
        })).collect::<Vec<_>>()
    });

    sqlx::query(
        "INSERT INTO verification_sessions (id, tenant_id, nonce, dcql_query, status, created_at, expires_at)
         VALUES ($1, $2, $3, $4, 'pending', NOW(), NOW() + INTERVAL '10 minutes')",
    )
    .bind(verification_session_id)
    .bind(auth.tenant_id)
    .bind(&nonce)
    .bind(dcql_query)
    .execute(&tenant_pool)
    .await?;

    let request_url =
        format!("openid4vp://authorize?session_id={verification_session_id}&nonce={nonce}");

    Ok(Json(CreatePresentationResponse {
        verification_session_id,
        request_url,
    }))
}
