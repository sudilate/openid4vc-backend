use axum::Json;
use axum::extract::{Form, Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header::CACHE_CONTROL};
use axum::response::IntoResponse;
use oid4vc_core::Validator;
use oid4vc_manager::storage::Storage;
use oid4vci::credential_request::{
    CredentialIdentifierOrCredentialConfigurationId, CredentialRequest,
};
use oid4vci::notification_request::NotificationRequest;
use oid4vci::token_request::TokenRequest;

use crate::error::AppError;
use crate::state::AppState;

pub async fn oauth_authorization_server(
    Path(tenant_slug): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let runtime = state.oid4vci_registry.get_by_slug(&tenant_slug).await?;
    Ok((
        StatusCode::OK,
        Json(
            runtime
                .manager
                .credential_issuer
                .authorization_server_metadata
                .clone(),
        ),
    ))
}

pub async fn openid_credential_issuer(
    Path(tenant_slug): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let runtime = state.oid4vci_registry.get_by_slug(&tenant_slug).await?;
    Ok((
        StatusCode::OK,
        Json(runtime.manager.credential_issuer.metadata.clone()),
    ))
}

pub async fn credential_offer(
    Path(tenant_slug): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let runtime = state.oid4vci_registry.get_by_slug(&tenant_slug).await?;
    let offer = runtime
        .manager
        .credential_offer()
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok((StatusCode::OK, Json(offer)))
}

pub async fn par(
    Path(tenant_slug): Path<String>,
    State(state): State<AppState>,
    Form(_request): Form<serde_json::Value>,
) -> Result<impl IntoResponse, AppError> {
    let runtime = state.oid4vci_registry.get_by_slug(&tenant_slug).await?;
    let response = runtime
        .manager
        .storage
        .get_pushed_authorization_response()
        .ok_or_else(|| {
            AppError::BadRequest("unable to build pushed authorization response".to_string())
        })?;

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn authorize(
    Path(tenant_slug): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let runtime = state.oid4vci_registry.get_by_slug(&tenant_slug).await?;
    let response = runtime
        .manager
        .storage
        .get_authorization_response()
        .ok_or_else(|| {
            AppError::BadRequest("unable to produce authorization response".to_string())
        })?;

    Ok((StatusCode::OK, Json(response)))
}

pub async fn token(
    Path(tenant_slug): Path<String>,
    State(state): State<AppState>,
    Form(request): Form<TokenRequest>,
) -> Result<impl IntoResponse, AppError> {
    let runtime = state.oid4vci_registry.get_by_slug(&tenant_slug).await?;
    let token = runtime
        .manager
        .storage
        .get_token_response(request)
        .ok_or_else(|| {
            AppError::BadRequest("invalid authorization or pre-authorized code".to_string())
        })?;

    let mut headers = HeaderMap::new();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));

    Ok((StatusCode::OK, headers, Json(token)))
}

pub async fn credential(
    Path(tenant_slug): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CredentialRequest>,
) -> Result<impl IntoResponse, AppError> {
    let runtime = state.oid4vci_registry.get_by_slug(&tenant_slug).await?;
    let access_token = bearer_token(&headers)?;

    let proofs = request
        .proofs
        .ok_or_else(|| AppError::BadRequest("missing proofs".to_string()))?;

    let validated_proofs = runtime
        .manager
        .credential_issuer
        .validate_proofs(
            proofs,
            Validator::Subject(runtime.manager.credential_issuer.subject.clone()),
        )
        .await
        .map_err(|err| AppError::BadRequest(format!("invalid proofs: {err}")))?;

    if validated_proofs.is_empty() {
        return Err(AppError::BadRequest("no valid proof supplied".to_string()));
    }

    let credential_configuration_id = match request
        .credential_identifier_or_credential_configuration_id
    {
        CredentialIdentifierOrCredentialConfigurationId::CredentialIdentifier(_) => {
            return Err(AppError::BadRequest(
                "credential_identifier flow is not implemented yet".to_string(),
            ));
        }
        CredentialIdentifierOrCredentialConfigurationId::CredentialConfigurationId(value) => value,
    };

    let subject_did = validated_proofs[0]
        .rfc7519_claims
        .iss()
        .clone()
        .unwrap_or_else(|| "did:example:subject".to_string())
        .parse()
        .map_err(|err| AppError::BadRequest(format!("invalid subject did in proof: {err}")))?;

    let response = runtime
        .manager
        .storage
        .get_credential_response(
            access_token,
            credential_configuration_id,
            subject_did,
            runtime
                .manager
                .credential_issuer
                .metadata
                .credential_issuer
                .clone(),
            runtime.manager.credential_issuer.subject.clone(),
        )
        .ok_or_else(|| AppError::BadRequest("unable to issue credential".to_string()))?;

    let mut headers = HeaderMap::new();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));

    Ok((StatusCode::OK, headers, Json(response)))
}

pub async fn notification(
    Path(_tenant_slug): Path<String>,
    headers: HeaderMap,
    Json(_request): Json<NotificationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let _token = bearer_token(&headers)?;
    Ok(StatusCode::NO_CONTENT)
}

fn bearer_token(headers: &HeaderMap) -> Result<String, AppError> {
    let raw = headers
        .get("authorization")
        .ok_or_else(|| AppError::Unauthorized("missing authorization header".to_string()))?
        .to_str()
        .map_err(|_| AppError::Unauthorized("invalid authorization header".to_string()))?;

    let token = raw
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("expected Bearer token".to_string()))?;

    Ok(token.to_string())
}
