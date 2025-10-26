use axum::{routing::get, Router};
use surevoucher_configcore::{AppConfig, Loader};
use std::net::SocketAddr;
use tracing::info;

#[cfg(feature = "tls")]
mod tls;

pub struct WebServer {
    cfg: AppConfig,
    router: Router,
}

impl WebServer {
    pub fn new(router: Router, cfg: AppConfig) -> Self {
        Self { cfg, router }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.cfg.host, self.cfg.port).parse()?;
        info!(%addr, "starting web server");

        let shutdown_signal = async {
            tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
            info!("shutdown signal received");
        };

        #[cfg(feature = "tls")]
        if let Some(tls) = self.cfg.tls.clone() {
            let listener = tls::make_tls_listener(&tls, addr).await?;
            axum::serve(listener, self.router)
                .with_graceful_shutdown(shutdown_signal)
                .await?;
            return Ok(());
        }

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, self.router)
            .with_graceful_shutdown(shutdown_signal)
            .await?;
        Ok(())
    }
}

pub fn basic_router() -> Router {
    Router::new().route("/healthz", get(|| async { "ok" }))
}

pub fn load_config() -> anyhow::Result<AppConfig> {
    let cfg = Loader::new("surevoucher", "SureVoucher", "SUREVOUCHER").load()?;
    Ok(cfg)
}