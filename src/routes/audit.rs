use std::convert::Infallible;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::Stream;
use serde_json::json;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::auth::AuthContext;
use crate::error::AppError;
use crate::state::AppState;

pub async fn stream(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let receiver = state.audit.subscribe();
    let stream = BroadcastStream::new(receiver).map(|result| {
        let event = match result {
            Ok(audit) => {
                let data = json!({
                    "id": audit.id,
                    "tenant_id": audit.tenant_id,
                    "principal": audit.principal,
                    "method": audit.method,
                    "path": audit.path,
                    "status_code": audit.status_code,
                    "latency_ms": audit.latency_ms,
                    "created_at": audit.created_at,
                });
                Event::default().event("audit").data(data.to_string())
            }
            Err(_) => Event::default().event("audit").data("{}"),
        };

        Ok(event)
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(10))))
}
