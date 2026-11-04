//! One browser connection: its own connection to the Java bridge, and every frame between the two.
//!
//! - Calls go to Java unchanged, to a Rust handler, or to both (routes, see `playmat_protocol::routes`).
//! - Java's replies and events come back unchanged, except game views when the gateway makes the state patches.
//! - The gateway watches logins on the connection, so Rust handlers know who calls.
//! - Closing either side closes the other, with the same close code (4001 "signed in elsewhere" included).

use crate::metrics::Metrics;
use crate::states::GameStates;
use crate::App;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http::HeaderMap;
use playmat_protocol::handlers::Caller;
use playmat_protocol::jsonrpc::{Outcome, Request};
use playmat_protocol::routes::Route;
use playmat_protocol::Access;
use serde_json::Value;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use yawc::close::CloseCode;
use yawc::{Frame, HttpRequestBuilder, HttpWebSocket, OpCode, Options, WebSocket};

/// headers a WebSocket handshake makes itself; everything else the browser sent goes on to Java (Origin, cookies,
/// the sign-on header a trusted proxy adds)
const HANDSHAKE_HEADERS: [&str; 10] = [
    "host",
    "connection",
    "upgrade",
    "content-length",
    "sec-websocket-key",
    "sec-websocket-version",
    "sec-websocket-extensions",
    "sec-websocket-accept",
    "x-forwarded-for",
    "x-real-ip",
];

/// what the gateway is waiting for in a reply from Java
enum Pending {
    /// a call routed to both: Java's answer goes to the client, the Rust one is compared with it
    Shadow { method: String, rust: Outcome },
    /// a login: on success, the connection's session (and its message ids) start again
    Login { method: String },
    /// setCapabilities with stateDiffs taken out (the gateway makes the patches): put it back in the reply
    Capabilities { wanted_diffs: bool },
    /// gameStateResync answered by the gateway, passed on to Java as gameResync to send the open question again
    Resync { had_state: bool },
    /// a call the gateway made itself; the reply is not passed on
    Whoami,
}

struct Shared {
    caller: Mutex<Caller>,
    states: Mutex<GameStates>,
    pending: Mutex<HashMap<String, Pending>>,
    queued_bytes: AtomicUsize,
}

fn id_key(id: &Value) -> String {
    id.to_string()
}

const WHOAMI_ID: &str = "gw:whoami";

/// Runs one connection until either side closes.
pub async fn run(app: Arc<App>, client: HttpWebSocket, peer: IpAddr, headers: HeaderMap) {
    let metrics = &app.metrics;
    Metrics::add(&metrics.connections_total, 1);
    metrics.connections_open.fetch_add(1, Ordering::Relaxed);
    let ip = crate::net::client_ip(
        peer,
        headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()),
        headers.get("x-real-ip").and_then(|v| v.to_str().ok()),
        &app.config.trusted_proxies,
    );
    let bucket = bucket_of(&ip, metrics.connections_total.load(Ordering::Relaxed));

    let trusted = app.config.trusted_proxies.iter().any(|proxy| proxy.contains(peer));
    let java = match connect_java(&app, &headers, ip, trusted).await {
        Ok(java) => java,
        Err(error) => {
            Metrics::add(&metrics.upstream_failures, 1);
            warn!("Java bridge unreachable for {ip}: {error}");
            let mut client = client;
            let _ = client.send(Frame::close(CloseCode::Again, "Server not ready")).await;
            metrics.connections_open.fetch_sub(1, Ordering::Relaxed);
            return;
        }
    };

    let shared = Arc::new(Shared {
        caller: Mutex::new(Caller { ip: ip.to_string(), ..Caller::default() }),
        states: Mutex::new(GameStates::new()),
        pending: Mutex::new(HashMap::new()),
        queued_bytes: AtomicUsize::new(0),
    });

    let (mut client_tx, mut client_rx) = client.split();
    let (mut java_tx, mut java_rx) = java.split();
    // frames for the client and for Java go through queues, so replies from Rust handlers, Java's frames and the
    // gateway's own messages are written by one task each
    let (to_client, mut client_queue) = mpsc::unbounded_channel::<Frame>();
    let (to_java, mut java_queue) = mpsc::unbounded_channel::<Frame>();

    let client_writer = {
        let shared = shared.clone();
        let app = app.clone();
        tokio::spawn(async move {
            while let Some(frame) = client_queue.recv().await {
                let len = frame.payload().len();
                let close = frame.opcode() == OpCode::Close;
                if client_tx.send(frame).await.is_err() {
                    break;
                }
                shared.queued_bytes.fetch_sub(len, Ordering::Relaxed);
                Metrics::add(&app.metrics.frames_to_client, 1);
                Metrics::add(&app.metrics.bytes_to_client, len as u64);
                if close {
                    break;
                }
            }
            let _ = client_tx.close().await;
        })
    };
    let java_writer = tokio::spawn(async move {
        while let Some(frame) = java_queue.recv().await {
            let close = frame.opcode() == OpCode::Close;
            if java_tx.send(frame).await.is_err() || close {
                break;
            }
        }
        let _ = java_tx.close().await;
    });

    let send_client = {
        let shared = shared.clone();
        let to_client = to_client.clone();
        let max = app.config.max_outgoing_bytes;
        let metrics_app = app.clone();
        move |text: String| -> bool {
            let len = text.len();
            let queued = shared.queued_bytes.fetch_add(len, Ordering::Relaxed) + len;
            if max > 0 && queued > max {
                Metrics::add(&metrics_app.metrics.slow_clients_dropped, 1);
                let _ = to_client.send(Frame::close(CloseCode::Policy, "Too slow"));
                return false;
            }
            to_client.send(Frame::text(Bytes::from(text))).is_ok()
        }
    };

    // Java -> client
    let from_java = {
        let shared = shared.clone();
        let app = app.clone();
        let to_client = to_client.clone();
        let to_java = to_java.clone();
        let send_client = send_client.clone();
        tokio::spawn(async move {
            while let Some(frame) = java_rx.next().await {
                match frame.opcode() {
                    OpCode::Text => {
                        let text = frame.as_str().to_string();
                        if !on_java_text(&app, &shared, &to_java, &send_client, text) {
                            break;
                        }
                    }
                    OpCode::Close => {
                        let code = frame.close_code().unwrap_or(CloseCode::Normal);
                        let reason = frame.close_reason().ok().flatten().unwrap_or("").to_string();
                        debug!("Java closed {ip}'s connection: {} {reason}", u16::from(code));
                        let _ = to_client.send(Frame::close(code, reason));
                        return;
                    }
                    _ => {}
                }
            }
            let _ = to_client.send(Frame::close(CloseCode::Away, "Server connection lost"));
        })
    };

    // client -> Java or Rust
    while let Some(frame) = client_rx.next().await {
        match frame.opcode() {
            OpCode::Text => {
                Metrics::add(&app.metrics.frames_from_client, 1);
                let text = frame.as_str().to_string();
                on_client_text(&app, &shared, bucket, &to_java, &send_client, text).await;
            }
            OpCode::Close => {
                let code = frame.close_code().unwrap_or(CloseCode::Normal);
                let _ = to_java.send(Frame::close(code, ""));
                break;
            }
            _ => {}
        }
    }
    let _ = to_java.send(Frame::close(CloseCode::Normal, ""));
    drop(to_java);
    let _ = java_writer.await;
    from_java.abort();
    // send_client and the Java reader hold senders too: a close frame ends the writer whoever still holds one
    let _ = to_client.send(Frame::close(CloseCode::Normal, ""));
    drop(send_client);
    drop(to_client);
    let _ = client_writer.await;
    let caller = shared.caller.lock().unwrap().clone();
    if caller.user.is_some() {
        app.handlers.session_ended(&caller);
    }
    app.metrics.connections_open.fetch_sub(1, Ordering::Relaxed);
}

/// 0–99, fixed for the connection: picks it in or out of a rollout
fn bucket_of(ip: &IpAddr, serial: u64) -> u8 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    ip.hash(&mut hasher);
    serial.hash(&mut hasher);
    (hasher.finish() % 100) as u8
}

/// Java believes the gateway (loopback) about forwarded headers, the sign-on header among them: only a trusted
/// proxy's are passed on, and from anyone else just the ones a browser sends
const UNTRUSTED_HEADERS: [&str; 3] = ["origin", "user-agent", "accept-language"];

async fn connect_java(
    app: &App,
    headers: &HeaderMap,
    ip: IpAddr,
    trusted: bool,
) -> anyhow::Result<WebSocket<yawc::MaybeTlsStream<tokio::net::TcpStream>>> {
    let mut request = HttpRequestBuilder::new();
    for (name, value) in headers {
        let pass = if trusted {
            !HANDSHAKE_HEADERS.contains(&name.as_str())
        } else {
            UNTRUSTED_HEADERS.contains(&name.as_str())
        };
        if pass {
            request = request.header(name, value);
        }
    }
    // Java trusts these from the gateway: they name the client the gateway believes
    request = request.header("X-Forwarded-For", ip.to_string()).header("X-Real-IP", ip.to_string());
    let options =
        Options::default().without_compression().with_limits(64 * 1024 * 1024, 64 * 1024 * 1024).with_no_delay();
    let java = WebSocket::connect(app.config.java_url.clone()).with_options(options).with_request(request).await?;
    Ok(java)
}

/// a frame from Java; false when the client is gone
fn on_java_text(
    app: &App,
    shared: &Shared,
    to_java: &mpsc::UnboundedSender<Frame>,
    send_client: &impl Fn(String) -> bool,
    text: String,
) -> bool {
    // replies start with "jsonrpc" (the bridge writes it first); anything else is an event
    if text.starts_with("{\"jsonrpc\"") {
        if shared.pending.lock().unwrap().is_empty() {
            return send_client(text);
        }
        let Ok(reply) = serde_json::from_str::<Value>(&text) else { return send_client(text) };
        let pending = reply.get("id").and_then(|id| shared.pending.lock().unwrap().remove(&id_key(id)));
        return match pending {
            None => send_client(text),
            Some(pending) => on_reply(app, shared, to_java, send_client, reply, text, pending),
        };
    }
    let diffs = app.config.state_diffs && shared.states.lock().unwrap().is_enabled();
    if !diffs {
        return send_client(text);
    }
    let Ok(Value::Object(event)) = serde_json::from_str::<Value>(&text) else { return send_client(text) };
    let mut states = shared.states.lock().unwrap();
    if let Some(id) = event.get("messageId").and_then(Value::as_i64) {
        states.sent(id);
    }
    match states.encode(&event) {
        None => send_client(text),
        Some(encoded) => {
            let metrics = &app.metrics;
            Metrics::add(if encoded.patch { &metrics.state_patches } else { &metrics.state_complete }, 1);
            Metrics::add(&metrics.state_full_bytes, encoded.full_chars as u64);
            Metrics::add(&metrics.state_sent_bytes, encoded.json.len() as u64);
            send_client(encoded.json)
        }
    }
}

fn on_reply(
    app: &App,
    shared: &Shared,
    to_java: &mpsc::UnboundedSender<Frame>,
    send_client: &impl Fn(String) -> bool,
    mut reply: Value,
    text: String,
    pending: Pending,
) -> bool {
    match pending {
        Pending::Shadow { method, rust } => {
            let java = Outcome::from_reply(&reply);
            let difference = match (&java, &rust) {
                (Some(Outcome::Result(a)), Outcome::Result(b)) => playmat_protocol::compare::difference(a, b),
                (Some(Outcome::Error { code: a, .. }), Outcome::Error { code: b, .. }) if a == b => None,
                _ => Some(format!(
                    "java {}, rust {}",
                    truncate(&format!("{java:?}"), 300),
                    truncate(&format!("{rust:?}"), 300)
                )),
            };
            app.metrics.shadow(&method, difference.is_none());
            if let Some(difference) = difference {
                warn!("shadow {method}: Rust answered differently at {difference}");
            }
            send_client(text)
        }
        Pending::Login { method } => {
            if reply.get("result") == Some(&Value::Bool(true)) {
                shared.states.lock().unwrap().new_session();
                {
                    let mut caller = shared.caller.lock().unwrap();
                    caller.admin = method == "connectAdmin";
                    caller.user = if caller.admin { Some("Admin".into()) } else { None };
                    caller.account = app.accounts.load(Ordering::Relaxed) && !caller.admin;
                }
                if method != "connectAdmin" {
                    shared.pending.lock().unwrap().insert(id_key(&Value::from(WHOAMI_ID)), Pending::Whoami);
                    let whoami = Request {
                        id: Some(Value::from(WHOAMI_ID)),
                        method: "sessionUserName".into(),
                        params: vec![Value::from("")],
                    };
                    let _ = to_java.send(Frame::text(Bytes::from(whoami.to_text())));
                }
            }
            send_client(text)
        }
        Pending::Capabilities { wanted_diffs } => {
            if let Some(Value::Array(on)) = reply.get_mut("result") {
                if wanted_diffs && !on.iter().any(|c| c.as_str() == Some("stateDiffs")) {
                    on.push(Value::from("stateDiffs"));
                }
            }
            shared.states.lock().unwrap().set_enabled(wanted_diffs);
            send_client(reply.to_string())
        }
        Pending::Resync { had_state } => {
            if had_state {
                if let Some(result) = reply.get_mut("result") {
                    *result = Value::Bool(true);
                }
            }
            send_client(reply.to_string())
        }
        Pending::Whoami => {
            if let Some(Value::String(user)) = reply.get("result") {
                shared.caller.lock().unwrap().user = Some(user.clone());
            }
            true
        }
    }
}

async fn on_client_text(
    app: &App,
    shared: &Shared,
    bucket: u8,
    to_java: &mpsc::UnboundedSender<Frame>,
    send_client: &impl Fn(String) -> bool,
    text: String,
) {
    if text.len() > app.config.max_message_bytes {
        // Java answers it (too large); pass it on so the error is the same
        let _ = to_java.send(Frame::text(Bytes::from(text)));
        return;
    }
    let Some(request) = Request::parse(&text) else {
        // not JSON, a batch, no method: the Java bridge gives the error
        let _ = to_java.send(Frame::text(Bytes::from(text)));
        return;
    };
    let info = playmat_protocol::method(&request.method);
    let gateway_diffs = app.config.state_diffs;

    // calls the gateway takes part in, whatever the route
    if let Some(id) = &request.id {
        if info.map(|m| m.access == Access::Establish).unwrap_or(false) {
            shared.pending.lock().unwrap().insert(id_key(id), Pending::Login { method: request.method.clone() });
        }
    }
    match request.method.as_str() {
        "setCapabilities" if gateway_diffs => {
            let asked: Vec<Value> = request.params.first().and_then(Value::as_array).cloned().unwrap_or_default();
            let wanted_diffs = asked.iter().any(|c| c.as_str() == Some("stateDiffs"));
            let rest: Vec<Value> = asked.into_iter().filter(|c| c.as_str() != Some("stateDiffs")).collect();
            match &request.id {
                Some(id) => {
                    shared.pending.lock().unwrap().insert(id_key(id), Pending::Capabilities { wanted_diffs });
                }
                None => shared.states.lock().unwrap().set_enabled(wanted_diffs),
            }
            let forwarded = Request { params: vec![Value::Array(rest)], ..request };
            Metrics::add(&app.metrics.requests_java, 1);
            let _ = to_java.send(Frame::text(Bytes::from(forwarded.to_text())));
            return;
        }
        "gameStateResync" if gateway_diffs && shared.states.lock().unwrap().is_enabled() => {
            let game_id = request.params.first().and_then(Value::as_str).unwrap_or_default().to_string();
            let resync = shared.states.lock().unwrap().resync(&game_id);
            let had_state = resync.is_some();
            if let Some(state) = resync {
                Metrics::add(&app.metrics.state_resyncs, 1);
                send_client(state);
            }
            if let Some(id) = &request.id {
                shared.pending.lock().unwrap().insert(id_key(id), Pending::Resync { had_state });
            }
            let forwarded = Request { method: "gameResync".into(), ..request };
            Metrics::add(&app.metrics.requests_java, 1);
            let _ = to_java.send(Frame::text(Bytes::from(forwarded.to_text())));
            return;
        }
        "gameWatchStop" | "replayStop" if gateway_diffs => {
            if let Some(game_id) = request.params.first().and_then(Value::as_str) {
                shared.states.lock().unwrap().forget(game_id);
            }
        }
        "disconnectSession" | "playerLogout" => {
            let caller = {
                let mut current = shared.caller.lock().unwrap();
                let ip = current.ip.clone();
                std::mem::replace(&mut *current, Caller { ip, ..Caller::default() })
            };
            if caller.user.is_some() {
                app.handlers.session_ended(&caller);
            }
        }
        _ => {}
    }

    let route = match app.config.routes.route(&request.method, bucket) {
        Route::Java => Route::Java,
        _ if !app.handlers.handles(&request.method) => Route::Java,
        route => route,
    };
    match route {
        Route::Java => {
            Metrics::add(&app.metrics.requests_java, 1);
            let _ = to_java.send(Frame::text(Bytes::from(text)));
        }
        Route::Rust => {
            Metrics::add(&app.metrics.requests_rust, 1);
            let caller = shared.caller.lock().unwrap().clone();
            let outcome = app.handlers.call(&caller, &request).await;
            if let Some(id) = &request.id {
                send_client(outcome.reply(id));
            }
        }
        Route::Shadow => {
            Metrics::add(&app.metrics.requests_shadow, 1);
            let caller = shared.caller.lock().unwrap().clone();
            let rust = app.handlers.call(&caller, &request).await;
            if let Some(id) = &request.id {
                shared
                    .pending
                    .lock()
                    .unwrap()
                    .insert(id_key(id), Pending::Shadow { method: request.method.clone(), rust });
            }
            let _ = to_java.send(Frame::text(Bytes::from(text)));
        }
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

pub fn log_start(app: &App) {
    info!(
        "Gateway on {} for the Java bridge at {}; state patches made {}; routes: {}",
        app.config.listen,
        app.config.java_url,
        if app.config.state_diffs { "here" } else { "by Java" },
        app.config.routes.describe()
    );
}
