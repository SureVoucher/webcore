use surevoucher_webcore::{basic_router, load_config, WebServer};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt().with_env_filter(EnvFilter::from_default_env()).init();
    let cfg = load_config()?;
    let app = basic_router();
    WebServer::new(app, cfg).run().await
}