use std::str::FromStr;

use axum::extract::FromRequestParts;
use http::request::Parts;
use uuid::Uuid;

use crate::auth::authorization::Role;
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub tenant_id: Uuid,
    pub role: Role,
    pub principal: String,
}

impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
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
            .map(parse_role)
            .transpose()?
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

fn parse_role(raw: &str) -> Result<Role, AppError> {
    match raw {
        "super_admin" => Ok(Role::SuperAdmin),
        "tenant_admin" => Ok(Role::TenantAdmin),
        "issuer_manager" => Ok(Role::IssuerManager),
        "verifier" => Ok(Role::Verifier),
        "readonly" => Ok(Role::ReadOnly),
        "api_client" => Ok(Role::ApiClient),
        _ => Err(AppError::Unauthorized(format!("invalid role: {raw}"))),
    }
}

impl FromStr for Role {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_role(s)
    }
}
