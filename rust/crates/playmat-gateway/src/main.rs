//! Playmat gateway (Rust migration phase 1, docs/RustMigration.md): owns the browser's WebSocket, passes each call
//! to the Java bridge or to a Rust handler by the routing table, and makes the game-state patches.
//!
//! HTTP on the same port: any path upgrades to the game connection (Caddy sends `/ws*` here); `GET /healthz` says
//! whether the Java bridge answers; `GET /metrics` gives the counters in Prometheus text format.

mod config;
mod connection;
mod metrics;
mod net;
mod states;

use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use config::Config;
use metrics::Metrics;
use playmat_protocol::handlers::Handlers;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};
use yawc::{IncomingUpgrade, Options};

pub struct App {
    pub config: Config,
    pub metrics: Metrics,
    pub handlers: Arc<dyn Handlers>,
    /// the server has accounts on (serverInfo.accounts), so a logged in user is signed in to one
    pub accounts: AtomicBool,
}

#[tokio::main]
async fn main() {
    // `playmat-gateway health`: for container health checks (exit 0 when /healthz says ok)
    if std::env::args().nth(1).as_deref() == Some("health") {
        std::process::exit(health_check().await);
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("XMAGE_GATEWAY_LOG").unwrap_or_else(|_| "info".into()),
        )
        .init();
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(problem) => {
            error!("{problem}");
            std::process::exit(2);
        }
    };
    // the platform service's calls (phase 2); which of them Rust answers is the routing table's choice
    let platform = playmat_platform::Platform::new(app_catalog(), app_decks().await);
    tokio::spawn(platform.clone().watch());
    let app = Arc::new(App {
        config,
        metrics: Metrics::default(),
        handlers: platform.clone(),
        accounts: AtomicBool::new(false),
    });
    connection::log_start(&app);
    tokio::spawn(watch_server_info(app.clone()));

    let router = Router::new()
        .route("/healthz", get(health))
        .route("/metrics", get(metrics))
        .fallback(upgrade)
        .with_state(app.clone());
    let listener = match tokio::net::TcpListener::bind(app.config.listen).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Can't listen on {}: {e}", app.config.listen);
            std::process::exit(1);
        }
    };
    let server = axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown());
    if let Err(e) = server.await {
        error!("Server stopped: {e}");
    }
    info!("Stopped; {}", app.metrics.render().lines().filter(|l| !l.starts_with('#')).collect::<Vec<_>>().join(", "));
}

/// `XMAGE_PLATFORM_DECKS`: the server's db/web_decks.db, for deck sync in Rust
async fn app_decks() -> Option<playmat_platform::decks::DeckStore> {
    let path: std::path::PathBuf = std::env::var("XMAGE_PLATFORM_DECKS").ok().filter(|v| !v.trim().is_empty())?.into();
    match playmat_platform::decks::DeckStore::open(&path).await {
        Ok(store) => {
            info!("Decks: {}", path.display());
            Some(store)
        }
        Err(e) => {
            warn!("Decks {} can't be opened, deck sync stays with Java: {e}", path.display());
            None
        }
    }
}

/// `XMAGE_PLATFORM_CATALOG`: the card catalog the server exports (`-Dxmage.catalog.export`)
fn app_catalog() -> Option<std::path::PathBuf> {
    std::env::var("XMAGE_PLATFORM_CATALOG").ok().filter(|v| !v.trim().is_empty()).map(Into::into)
}

async fn health_check() -> i32 {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listen = std::env::var("XMAGE_GATEWAY_LISTEN").unwrap_or_else(|_| "0.0.0.0:17180".into());
    let port = listen.rsplit(':').next().unwrap_or("17180");
    let check = async {
        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}")).await.ok()?;
        stream.write_all(b"GET /healthz HTTP/1.0\r\n\r\n").await.ok()?;
        let mut head = [0u8; 12];
        stream.read_exact(&mut head).await.ok()?;
        Some(head.ends_with(b"200"))
    };
    match tokio::time::timeout(Duration::from_secs(5), check).await {
        Ok(Some(true)) => 0,
        _ => 1,
    }
}

async fn shutdown() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("signal");
        tokio::select! {
            _ = ctrl_c => {},
            _ = term.recv() => {},
        }
    }
    #[cfg(not(unix))]
    let _ = ctrl_c.await;
}

async fn upgrade(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    upgrade: Result<IncomingUpgrade, StatusCode>,
) -> Response {
    let Ok(upgrade) = upgrade else {
        return (StatusCode::BAD_REQUEST, "WebSocket only\n").into_response();
    };
    let options = Options::default()
        .with_low_latency_compression()
        .with_limits(app.config.max_message_bytes, app.config.max_message_bytes)
        .with_no_delay();
    let (response, socket) = match upgrade.upgrade(options) {
        Ok(upgrade) => upgrade,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("{e}\n")).into_response(),
    };
    tokio::spawn(async move {
        match socket.await {
            Ok(client) => connection::run(app, client, peer.ip(), headers).await,
            Err(e) => warn!("Handshake with {peer} failed: {e}"),
        }
    });
    response.into_response()
}

async fn health(State(app): State<Arc<App>>) -> Response {
    let java = app.config.java_url.socket_addrs(|| Some(80)).ok().and_then(|a| a.first().copied());
    let reachable = match java {
        Some(addr) => tokio::time::timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(addr))
            .await
            .map(|r| r.is_ok())
            .unwrap_or(false),
        None => false,
    };
    let open = app.metrics.connections_open.load(Ordering::Relaxed);
    if reachable {
        (StatusCode::OK, format!("ok {open} connections\n")).into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "Java bridge unreachable\n").into_response()
    }
}

async fn metrics(State(app): State<Arc<App>>) -> Response {
    ([(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")], app.metrics.render()).into_response()
}

/// keeps `accounts` in step with the server (it can change with the admin console's settings)
async fn watch_server_info(app: Arc<App>) {
    use futures_util::{SinkExt, StreamExt};
    loop {
        let result: anyhow::Result<bool> = async {
            let mut java =
                yawc::WebSocket::connect(app.config.java_url.clone()).with_options(Options::default()).await?;
            java.send(yawc::Frame::text(r#"{"jsonrpc":"2.0","id":1,"method":"serverInfo","params":[]}"#)).await?;
            while let Some(frame) = java.next().await {
                if frame.opcode() == yawc::OpCode::Text {
                    let reply: serde_json::Value = serde_json::from_str(frame.as_str())?;
                    if reply.get("id") == Some(&serde_json::Value::from(1)) {
                        let _ = java.close().await;
                        return Ok(reply["result"]["accounts"].as_bool().unwrap_or(false));
                    }
                }
            }
            anyhow::bail!("no serverInfo reply")
        }
        .await;
        match result {
            Ok(accounts) => {
                if app.accounts.swap(accounts, Ordering::Relaxed) != accounts {
                    info!("Server accounts: {}", if accounts { "on" } else { "off" });
                }
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
            Err(_) => tokio::time::sleep(Duration::from_secs(5)).await,
        }
    }
}
