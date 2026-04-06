use axum::extract::{FromRef, FromRequestParts};
use http::request::Parts;
use uuid::Uuid;

use crate::auth::api_key::verify_api_key;
use crate::auth::authorization::{Role, role_from_str};
use crate::auth::jwt::decode_and_validate_jwt;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub tenant_id: Uuid,
    pub role: Role,
    pub principal: String,
}

impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        if let Some(auth_header) = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|raw| raw.strip_prefix("Bearer "))
        {
            let claims = decode_and_validate_jwt(
                auth_header,
                &app_state.settings.security.jwt_public_key_pem,
                &app_state.settings.security.jwt_issuer,
                &app_state.settings.security.jwt_audience,
            )
            .map_err(|err| AppError::Unauthorized(format!("invalid jwt token: {err}")))?;

            let role = role_from_str(&claims.role)
                .ok_or_else(|| AppError::Unauthorized("invalid role in jwt token".to_string()))?;

            return Ok(Self {
                tenant_id: claims.tenant_id,
                role,
                principal: claims.sub,
            });
        }

        if let Some(api_key) = parts.headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
            let row = sqlx::query_as::<_, (Uuid, String, String, Uuid)>(
                "SELECT tenant_id, role, key_hash, id FROM api_keys WHERE is_active = true",
            )
            .fetch_all(&app_state.admin_db)
            .await?;

            for (tenant_id, role_raw, key_hash, key_id) in row {
                if verify_api_key(api_key, &key_hash) {
                    let role = role_from_str(&role_raw).unwrap_or(Role::ApiClient);

                    let _ = sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
                        .bind(key_id)
                        .execute(&app_state.admin_db)
                        .await;

                    return Ok(Self {
                        tenant_id,
                        role,
                        principal: "api_key".to_string(),
                    });
                }
            }

            return Err(AppError::Unauthorized("invalid api key".to_string()));
        }

        // Local development fallback.
        let tenant_id = parts
            .headers
            .get("x-tenant-id")
            .ok_or_else(|| AppError::Unauthorized("missing x-tenant-id header".to_string()))?
            .to_str()
            .map_err(|_| AppError::Unauthorized("invalid x-tenant-id header".to_string()))?
            .parse::<Uuid>()
            .map_err(|_| AppError::Unauthorized("invalid tenant id".to_string()))?;

        let role = parts
            .headers
            .get("x-role")
            .and_then(|v| v.to_str().ok())
            .and_then(role_from_str)
            .unwrap_or(Role::ReadOnly);

        let principal = parts
            .headers
            .get("x-principal")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        Ok(Self {
            tenant_id,
            role,
            principal,
        })
    }
}
