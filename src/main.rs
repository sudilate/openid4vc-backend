use std::net::SocketAddr;

use openid4vc_backend::app::build_app;
use openid4vc_backend::config::Settings;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let settings = Settings::from_env()?;
    let addr: SocketAddr = format!("{}:{}", settings.server.host, settings.server.port)
        .parse()
        .map_err(|err| anyhow::anyhow!("invalid bind address: {err}"))?;
    let app = build_app(settings).await?;

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("openid4vc-backend listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
