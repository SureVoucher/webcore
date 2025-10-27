use axum::{routing::get, Router};
use surevoucher_configcore::{AppConfig, Loader};
use std::net::SocketAddr;
use tracing::{info, warn};

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
        let app_addr: SocketAddr = format!("{}:{}", self.cfg.host, self.cfg.port).parse()?;
        info!(%app_addr, "starting app server");

        // Start health server on separate port if configured (defaults to 127.0.0.1:18080)
        let health_host = std::env::var("SUREVOUCHER__HEALTH_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let health_port: u16 = std::env::var("SUREVOUCHER__HEALTH_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(18080);
        let health_addr: SocketAddr = format!("{}:{}", health_host, health_port).parse()?;

        let health_app = Router::new().route("/healthz", get(|| async { "ok" }));
        let health_task = tokio::spawn(async move {
            match tokio::net::TcpListener::bind(health_addr).await {
                Ok(listener) => {
                    info!(%health_addr, "health server listening");
                    if let Err(e) = axum::serve(listener, health_app).await {
                        warn!(error = %e, "health server terminated");
                    }
                }
                Err(e) => warn!(error = %e, %health_addr, "failed to bind health server"),
            }
        });

        // graceful shutdown (Ctrl+C)
        let shutdown_signal = async {
            tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
            info!("shutdown signal received");
        };

        #[cfg(feature = "tls")]
        if let Some(tls) = self.cfg.tls.clone() {
            let listener = tls::make_tls_listener(&tls, app_addr).await?;
            axum::serve(listener, self.router)
                .with_graceful_shutdown(shutdown_signal)
                .await?;
            health_task.abort();
            return Ok(());
        }

        let listener = tokio::net::TcpListener::bind(app_addr).await?;
        axum::serve(listener, self.router)
            .with_graceful_shutdown(shutdown_signal)
            .await?;
        health_task.abort();
        Ok(())
    }
}

pub fn basic_router() -> Router {
    // Only core app routes here; health lives on the separate health server
    Router::new().route("/", get(|| async { "ok" })) // a placeholder root handler for embedding apps
}

pub fn load_config() -> anyhow::Result<AppConfig> {
    let cfg = Loader::new("surevoucher", "SureVoucher", "SUREVOUCHER").load()?;
    Ok(cfg)
}