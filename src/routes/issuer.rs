use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::auth::authorization::{Action, Resource};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateOfferRequest {
    pub credential_definition_id: Uuid,
    pub by_reference: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateCredentialDefinitionRequest {
    pub name: String,
    pub format: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct CreateOfferResponse {
    pub issuance_session_id: Uuid,
    pub offer_url: String,
}

#[derive(Debug, Serialize)]
pub struct CreateCredentialDefinitionResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct RevokeCredentialRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RevokeCredentialResponse {
    pub credential_id: String,
    pub status: String,
}

pub async fn create_credential_definition(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<CreateCredentialDefinitionRequest>,
) -> Result<Json<CreateCredentialDefinitionResponse>, AppError> {
    if !state
        .authorization
        .is_allowed(auth.role, Resource::CredentialDefinition, Action::Create)
    {
        return Err(AppError::Forbidden(
            "role is not allowed to create credential definitions".to_string(),
        ));
    }

    let definition_id = Uuid::new_v4();
    let tenant_pool = state.tenant_pools.pool_for_tenant(auth.tenant_id).await?;

    sqlx::query(
        "INSERT INTO credential_definitions (id, tenant_id, name, format, schema, is_active, created_at)
         VALUES ($1, $2, $3, $4, $5, true, NOW())",
    )
    .bind(definition_id)
    .bind(auth.tenant_id)
    .bind(&payload.name)
    .bind(&payload.format)
    .bind(payload.schema)
    .execute(&tenant_pool)
    .await?;

    Ok(Json(CreateCredentialDefinitionResponse {
        id: definition_id,
        tenant_id: auth.tenant_id,
        name: payload.name,
    }))
}

pub async fn create_credential_offer(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<CreateOfferRequest>,
) -> Result<Json<CreateOfferResponse>, AppError> {
    if !state
        .authorization
        .is_allowed(auth.role, Resource::IssuanceSession, Action::Create)
    {
        return Err(AppError::Forbidden(
            "role is not allowed to create issuance sessions".to_string(),
        ));
    }

    let issuance_session_id = Uuid::new_v4();
    let tenant_pool = state.tenant_pools.pool_for_tenant(auth.tenant_id).await?;
    let pre_authorized_code = Uuid::new_v4().to_string();

    let tenant_slug = sqlx::query_scalar::<_, String>("SELECT slug FROM tenants WHERE id = $1")
        .bind(auth.tenant_id)
        .fetch_one(&state.admin_db)
        .await?;
    state
        .oid4vci_registry
        .set_pre_authorized_code(&tenant_slug, pre_authorized_code.clone())
        .await?;

    sqlx::query(
        "INSERT INTO issuance_sessions (id, tenant_id, credential_definition_id, pre_authorized_code, flow_type, status, created_at, expires_at)
         VALUES ($1, $2, $3, $4, 'pre_authorized_code', 'pending', NOW(), NOW() + INTERVAL '1 hour')",
    )
    .bind(issuance_session_id)
    .bind(auth.tenant_id)
    .bind(payload.credential_definition_id)
    .bind(&pre_authorized_code)
    .execute(&tenant_pool)
    .await?;

    let offer_url = if payload.by_reference {
        format!(
            "openid-credential-offer://?credential_offer_uri=https://api.local/oid4vci/{tenant_slug}/credential_offer"
        )
    } else {
        format!(
            "openid-credential-offer://?credential_offer={{\"pre-authorized_code\":\"{pre_authorized_code}\"}}"
        )
    };

    Ok(Json(CreateOfferResponse {
        issuance_session_id,
        offer_url,
    }))
}

pub async fn revoke_issued_credential(
    Path(credential_id): Path<String>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<RevokeCredentialRequest>,
) -> Result<Json<RevokeCredentialResponse>, AppError> {
    if !state
        .authorization
        .is_allowed(auth.role, Resource::IssuedCredential, Action::Revoke)
    {
        return Err(AppError::Forbidden(
            "role is not allowed to revoke credentials".to_string(),
        ));
    }

    let tenant_pool = state.tenant_pools.pool_for_tenant(auth.tenant_id).await?;
    let revocation_service = crate::services::revocation::RevocationService::new(tenant_pool);
    revocation_service
        .revoke_credential(
            auth.tenant_id,
            &credential_id,
            payload.reason,
            Some(auth.principal),
        )
        .await?;

    Ok(Json(RevokeCredentialResponse {
        credential_id,
        status: "revoked".to_string(),
    }))
}
