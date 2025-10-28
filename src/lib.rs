// 1) Re-export axum so clients avoid version skew
pub use axum;
use axum::{routing::get, Router};
use surevoucher_configcore::{AppConfig, Loader};
use std::{net::SocketAddr, sync::atomic::{AtomicBool, Ordering}};
use once_cell::sync::Lazy;
use tracing::{info, warn};
use tokio::signal;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::{Arc, RwLock};


#[cfg(unix)]
use tokio::signal::unix::{signal as unix_signal, SignalKind};

static READY: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

static PROM_HANDLE: Lazy<PrometheusHandle> = Lazy::new(|| {
    PrometheusBuilder::new()
        .install_recorder()
        .expect("install prometheus recorder")
});

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
        init_logging();

        let app_addr: SocketAddr = format!("{}:{}", self.cfg.host, self.cfg.port).parse()?;

        let health_host = std::env::var("SUREVOUCHER__HEALTH_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let health_port: u16 = std::env::var("SUREVOUCHER__HEALTH_PORT").ok()
            .and_then(|s| s.parse::<u16>().ok()).unwrap_or(18080);
        let health_addr: SocketAddr = format!("{}:{}", health_host, health_port).parse()?;

        let health_task = tokio::spawn(async move {
            let health_app = health_router();
            match tokio::net::TcpListener::bind(health_addr).await {
                Ok(listener) => {
                    info!(addr=%health_addr, "health server listening");
                    if let Err(e) = axum::serve(listener, health_app).await {
                        warn!(error=%e, "health server terminated");
                    }
                }
                Err(e) => warn!(addr=%health_addr, error=%e, "failed to bind health server"),
            }
        });

        let shutdown = shutdown_signal();

        READY.store(true, Ordering::SeqCst);

        #[cfg(feature = "tls")]
        if let Some(tls_cfg) = self.cfg.tls.clone() {
            let listener = tls::make_tls_listener(&tls_cfg, app_addr).await?;
            axum::serve(listener, self.router)
                .with_graceful_shutdown(shutdown)
                .await?;
            health_task.abort();
            return Ok(());
        }

        let listener = tokio::net::TcpListener::bind(app_addr).await?;
        info!(addr=%app_addr, "app server listening");
        axum::serve(listener, self.router)
            .with_graceful_shutdown(shutdown)
            .await?;

        health_task.abort();
        Ok(())
    }
}

pub fn load_config() -> anyhow::Result<AppConfig> {
    let cfg = Loader::new("surevoucher", "SureVoucher", "SUREVOUCHER").load()?;
    Ok(cfg)
}

fn init_logging() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .flatten_event(true)
        .with_current_span(true)
        .with_span_list(true)
        .try_init();
}

fn shutdown_signal() -> impl std::future::Future<Output = ()> {
    async move {
        let ctrl_c = async {
            if let Err(e) = signal::ctrl_c().await {
                warn!(error=%e, "failed to install Ctrl-C handler");
            }
        };
        #[cfg(unix)]
        {
            let mut term = unix_signal(SignalKind::terminate()).expect("sigterm");
            let mut quit = unix_signal(SignalKind::quit()).expect("sigquit");
            let mut hup  = unix_signal(SignalKind::hangup()).expect("sighup");

            tokio::select! {
                _ = ctrl_c => {},
                _ = term.recv() => {},
                _ = quit.recv() => {},
                _ = hup.recv()  => {},
            }
        }
        #[cfg(not(unix))]
        {
            ctrl_c.await;
        }
        info!("shutdown signal received");
    }
}

fn health_router() -> Router {
    use axum::response::IntoResponse;

    async fn healthz() -> &'static str { "ok" }
    async fn ready() -> &'static str {
        if READY.load(Ordering::SeqCst) { "ok" } else { "starting" }
    }

    // Axum handler
    async fn metrics() -> impl IntoResponse {
        PROM_HANDLE.render()
    }

    Router::new()
        .route("/healthz", get(healthz))
        .route("/ready",   get(ready))
        .route("/metrics", get(metrics))
}

static GLOBAL_ROUTER: Lazy<Arc<RwLock<Router<()>>>> =
    Lazy::new(|| Arc::new(RwLock::new(Router::new().route("/healthz", get(|| async { "ok" })))));

pub fn basic_router() -> Router<()> {
    GLOBAL_ROUTER.read().unwrap().clone()
}

/// Add a new route to the global router
pub fn add_route(path: &'static str, route: axum::routing::MethodRouter<()>) {
    let mut router = GLOBAL_ROUTER.write().unwrap();
    *router = router.clone().route(path, route);
}

/// Exported run function for servers
pub async fn run() -> anyhow::Result<()> {
    let router = GLOBAL_ROUTER.read().unwrap().clone();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, router).await?;
    Ok(())
}