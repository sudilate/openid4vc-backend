use axum::http::StatusCode;

use crate::services::metrics;

pub async fn metrics() -> (StatusCode, String) {
    (StatusCode::OK, metrics::encode_metrics())
}
