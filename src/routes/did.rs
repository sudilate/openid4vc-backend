use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ResolveDidQuery {
    pub did: String,
}

#[derive(Debug, Serialize)]
pub struct ResolveDidResponse {
    pub did: String,
    pub document: serde_json::Value,
}

pub async fn resolve_did(
    State(state): State<AppState>,
    Query(query): Query<ResolveDidQuery>,
) -> Result<Json<ResolveDidResponse>, AppError> {
    let document = state.did_registry.resolve(&query.did).await?;
    Ok(Json(ResolveDidResponse {
        did: query.did,
        document,
    }))
}
