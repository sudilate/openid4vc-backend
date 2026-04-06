pub mod audit;
pub mod did;
pub mod health;
pub mod issuer;
pub mod keys;
pub mod metrics;
pub mod openid4vci;
pub mod tenant;
pub mod verifier;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .route("/metrics", get(metrics::metrics))
        .route("/api/v1/audit/stream", get(audit::stream))
        .route("/api/v1/tenants", post(tenant::create_tenant))
        .route(
            "/api/v1/tenants/{tenant_id}/api-keys",
            post(tenant::create_api_key),
        )
        .route("/api/v1/did/resolve", get(did::resolve_did))
        .route("/api/v1/keys/rotate", post(keys::rotate_primary_key))
        .route(
            "/api/v1/issuer/offers",
            post(issuer::create_credential_offer),
        )
        .route(
            "/api/v1/issuer/credential-definitions",
            post(issuer::create_credential_definition),
        )
        .route(
            "/api/v1/issuer/issued/{credential_id}/revoke",
            post(issuer::revoke_issued_credential),
        )
        .route(
            "/api/v1/verifier/requests",
            post(verifier::create_presentation_request),
        )
        .route(
            "/oid4vci/{tenant_slug}/.well-known/oauth-authorization-server",
            get(openid4vci::oauth_authorization_server),
        )
        .route(
            "/oid4vci/{tenant_slug}/.well-known/openid-credential-issuer",
            get(openid4vci::openid_credential_issuer),
        )
        .route(
            "/oid4vci/{tenant_slug}/credential_offer",
            get(openid4vci::credential_offer),
        )
        .route("/oid4vci/{tenant_slug}/par", post(openid4vci::par))
        .route(
            "/oid4vci/{tenant_slug}/authorize",
            get(openid4vci::authorize),
        )
        .route("/oid4vci/{tenant_slug}/token", post(openid4vci::token))
        .route(
            "/oid4vci/{tenant_slug}/credential",
            post(openid4vci::credential),
        )
        .route(
            "/oid4vci/{tenant_slug}/notification",
            post(openid4vci::notification),
        )
        .with_state(state)
}
