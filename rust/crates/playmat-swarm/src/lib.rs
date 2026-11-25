//! Mana@Home's coordinator (docs/swarm/Design.md): leases work units to workers, presigns their uploads, checks a
//! result against its lease (intake), and hands completed units to the trainer, which replays them and sends a
//! verdict. It keeps metadata only, in SQLite: game data goes from the worker to the store and from the store to the
//! trainer. The API is `playmat_swarm_proto::api`.
//!
//! S1 scope: tokens are issued by an admin (`playmat-swarm token`), not tied to Playmat accounts yet; acceptance is
//! the trainer's replay of every game. Re-run audits, reputation and credit come in S2.

pub mod store;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use playmat_swarm_proto::api::*;
use playmat_swarm_proto::{self as proto, ResultManifest, WorkUnit, PROTOCOL};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use std::sync::Arc;
use store::{now, Store};

/// How long a lease lasts without a heartbeat, and how often an idle worker asks again
pub const LEASE_SECS: u64 = 900;
pub const POLL_SECS: u64 = 60;
/// Leases a unit gets before it's marked failed
pub const MAX_ATTEMPTS: i64 = 5;
/// Presigned URLs' lifetime
const URL_SECS: u64 = 3600;
/// The largest file a worker may upload (a 100-game unit's biggest column is a few MB)
const MAX_FILE: u64 = 256 << 20;

pub struct Swarm {
    pub db: SqlitePool,
    pub store: Store,
    /// the coordinator's own URL, for the networks' download links
    pub base: String,
    /// one lease at a time, so two workers never get the same unit
    lease: tokio::sync::Mutex<()>,
}

pub type Shared = Arc<Swarm>;

const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS tokens (id INTEGER PRIMARY KEY, hash TEXT NOT NULL UNIQUE, role TEXT NOT NULL, \
     name TEXT NOT NULL, created INTEGER NOT NULL, revoked INTEGER NOT NULL DEFAULT 0)",
    "CREATE TABLE IF NOT EXISTS hosts (id INTEGER PRIMARY KEY, token INTEGER NOT NULL, name TEXT NOT NULL, \
     build TEXT NOT NULL, class TEXT NOT NULL, strict INTEGER NOT NULL, threads INTEGER NOT NULL, os TEXT NOT NULL, \
     seen INTEGER NOT NULL, UNIQUE(token, name))",
    "CREATE TABLE IF NOT EXISTS rounds (id INTEGER PRIMARY KEY, project TEXT NOT NULL, run TEXT NOT NULL, \
     round INTEGER NOT NULL, priority INTEGER NOT NULL, created INTEGER NOT NULL, UNIQUE(project, run, round))",
    "CREATE TABLE IF NOT EXISTS units (id TEXT PRIMARY KEY, round INTEGER NOT NULL, unit TEXT NOT NULL, \
     state TEXT NOT NULL, host INTEGER, lease_until INTEGER, attempts INTEGER NOT NULL DEFAULT 0, manifest TEXT, \
     games INTEGER NOT NULL DEFAULT 0, done_at INTEGER, verdict TEXT)",
    "CREATE INDEX IF NOT EXISTS units_state ON units(state, round)",
    "CREATE TABLE IF NOT EXISTS uploads (unit TEXT NOT NULL, path TEXT NOT NULL, size INTEGER NOT NULL, \
     sha256 TEXT NOT NULL, PRIMARY KEY(unit, path))",
    // a host that gave a unit back never gets it again: a broken host can't use up a unit's attempts alone
    "CREATE TABLE IF NOT EXISTS failures (unit TEXT NOT NULL, host INTEGER NOT NULL, PRIMARY KEY(unit, host))",
    "CREATE TABLE IF NOT EXISTS audit (id INTEGER PRIMARY KEY, at INTEGER NOT NULL, actor TEXT NOT NULL, \
     action TEXT NOT NULL, detail TEXT NOT NULL)",
];

impl Swarm {
    pub async fn open(db_path: &str, store: Store, base: &str) -> anyhow::Result<Shared> {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite:{db_path}"))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(10));
        let db = SqlitePoolOptions::new().max_connections(4).connect_with(opts).await?;
        for s in SCHEMA {
            sqlx::query(s).execute(&db).await?;
        }
        Ok(Arc::new(Swarm {
            db,
            store,
            base: base.trim_end_matches('/').to_string(),
            lease: tokio::sync::Mutex::new(()),
        }))
    }

    /// Makes a token and keeps its hash; the token itself is shown once
    pub async fn new_token(&self, role: Role, name: &str) -> anyhow::Result<TokenReply> {
        let mut raw = [0u8; 24];
        getrandom::getrandom(&mut raw).map_err(|e| anyhow::anyhow!("randomness: {e}"))?;
        let token = format!("mana_{}_{}", role.name(), hex(&raw));
        let id = sqlx::query("INSERT INTO tokens (hash, role, name, created) VALUES (?, ?, ?, ?)")
            .bind(hash(&token))
            .bind(role.name())
            .bind(name)
            .bind(now() as i64)
            .execute(&self.db)
            .await?
            .last_insert_rowid();
        self.audit("cli", "token", &format!("{} token {id} for {name}", role.name())).await;
        Ok(TokenReply { id, token })
    }

    async fn audit(&self, actor: &str, action: &str, detail: &str) {
        let _ = sqlx::query("INSERT INTO audit (at, actor, action, detail) VALUES (?, ?, ?, ?)")
            .bind(now() as i64)
            .bind(actor)
            .bind(action)
            .bind(detail)
            .execute(&self.db)
            .await;
    }
}

/// Serves the API (and the local store's routes) on `listener` until the process ends
pub async fn serve(listener: tokio::net::TcpListener, s: Shared) -> std::io::Result<()> {
    axum::serve(listener, router(s)).await
}

pub fn router(s: Shared) -> Router {
    Router::new()
        .route("/api/v1/status", get(status))
        .route("/api/v1/tokens", post(new_token))
        .route("/api/v1/hello", post(hello))
        .route("/api/v1/lease", post(lease))
        .route("/api/v1/units/{id}/heartbeat", post(heartbeat))
        .route("/api/v1/units/{id}/uploads", post(uploads))
        .route("/api/v1/units/{id}/complete", post(complete))
        .route("/api/v1/units/{id}/fail", post(fail))
        .route("/api/v1/units/{id}/release", post(release))
        .route("/api/v1/units/{id}/verdict", post(verdict))
        .route("/api/v1/nets/{sha}", get(get_net).post(put_net))
        .route("/api/v1/rounds", post(new_round))
        .route("/api/v1/rounds/{id}", get(round_status))
        .route("/api/v1/rounds/{id}/results", get(round_results))
        .route("/store/{*key}", get(store_get).put(store_put).layer(DefaultBodyLimit::max(MAX_FILE as usize)))
        .with_state(s)
}

// ---- errors and auth

pub struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> ApiError {
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("database: {e}"))
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> ApiError {
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    }
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

fn conflict(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::CONFLICT, msg.into())
}

type Res<T> = Result<T, ApiError>;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hash(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
}

/// Who called: the token's id, role and name
struct Caller {
    token: i64,
    name: String,
}

async fn caller(s: &Swarm, headers: &HeaderMap, roles: &[Role]) -> Res<Caller> {
    let unauthorized = || ApiError(StatusCode::UNAUTHORIZED, "a valid bearer token is needed".into());
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(unauthorized)?;
    let row = sqlx::query("SELECT id, role, name FROM tokens WHERE hash = ? AND revoked = 0")
        .bind(hash(token))
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(unauthorized)?;
    let role = Role::parse(row.get::<String, _>("role").as_str()).ok_or_else(unauthorized)?;
    if !roles.contains(&role) {
        return Err(ApiError(StatusCode::FORBIDDEN, format!("a {} token can't do that", role.name())));
    }
    Ok(Caller { token: row.get("id"), name: row.get("name") })
}

// ---- public

async fn status(State(s): State<Shared>) -> Res<Json<Status>> {
    let since = now() as i64 - 600;
    let row = sqlx::query("SELECT COUNT(*) AS n, COALESCE(SUM(threads), 0) AS t FROM hosts WHERE seen >= ?")
        .bind(since)
        .fetch_one(&s.db)
        .await?;
    let mut st =
        Status { hosts: row.get::<i64, _>("n") as u32, threads: row.get::<i64, _>("t") as u32, ..Default::default() };
    for r in sqlx::query("SELECT state, COUNT(*) AS n FROM units GROUP BY state").fetch_all(&s.db).await? {
        st.units.insert(r.get("state"), r.get::<i64, _>("n") as u32);
    }
    st.games_accepted = sqlx::query("SELECT COALESCE(SUM(games), 0) AS g FROM units WHERE state = 'accepted'")
        .fetch_one(&s.db)
        .await?
        .get::<i64, _>("g") as u64;
    st.games_last_hour = sqlx::query(
        "SELECT COALESCE(SUM(games), 0) AS g FROM units WHERE state IN ('complete', 'accepted') AND done_at >= ?",
    )
    .bind(now() as i64 - 3600)
    .fetch_one(&s.db)
    .await?
    .get::<i64, _>("g") as u64;
    Ok(Json(st))
}

// ---- admin

async fn new_token(State(s): State<Shared>, headers: HeaderMap, Json(req): Json<NewToken>) -> Res<Json<TokenReply>> {
    let c = caller(&s, &headers, &[Role::Admin]).await?;
    let reply = s.new_token(req.role, &req.name).await?;
    s.audit(&c.name, "token", &format!("{} token {} for {}", req.role.name(), reply.id, req.name)).await;
    Ok(Json(reply))
}

// ---- worker

async fn hello(State(s): State<Shared>, headers: HeaderMap, Json(h): Json<Hello>) -> Res<Json<HelloReply>> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    if h.name.is_empty() || h.name.len() > 64 {
        return Err(bad("a host name of 1 to 64 characters"));
    }
    let id: i64 = sqlx::query(
        "INSERT INTO hosts (token, name, build, class, strict, threads, os, seen) VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(token, name) DO UPDATE SET build = excluded.build, class = excluded.class, \
         strict = excluded.strict, threads = excluded.threads, os = excluded.os, seen = excluded.seen RETURNING id",
    )
    .bind(c.token)
    .bind(&h.name)
    .bind(&h.build)
    .bind(&h.class)
    .bind(h.strict)
    .bind(h.threads as i64)
    .bind(&h.os)
    .bind(now() as i64)
    .fetch_one(&s.db)
    .await?
    .get("id");
    Ok(Json(HelloReply { host: id, poll_secs: POLL_SECS }))
}

/// A host of the caller's token: (build, strict)
async fn own_host(s: &Swarm, c: &Caller, host: i64) -> Res<(String, bool)> {
    let row = sqlx::query("SELECT build, strict FROM hosts WHERE id = ? AND token = ?")
        .bind(host)
        .bind(c.token)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| bad("no such host on this token: say hello first"))?;
    sqlx::query("UPDATE hosts SET seen = ? WHERE id = ?").bind(now() as i64).bind(host).execute(&s.db).await?;
    Ok((row.get("build"), row.get::<i64, _>("strict") != 0))
}

async fn lease(State(s): State<Shared>, headers: HeaderMap, Json(req): Json<LeaseRequest>) -> Res<Response> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    let (build, strict) = own_host(&s, &c, req.host).await?;
    let _one = s.lease.lock().await;
    let t = now() as i64;
    // expired leases go back to the queue, or fail after MAX_ATTEMPTS
    sqlx::query("UPDATE units SET state = 'failed' WHERE state = 'leased' AND lease_until < ? AND attempts >= ?")
        .bind(t)
        .bind(MAX_ATTEMPTS)
        .execute(&s.db)
        .await?;
    sqlx::query("UPDATE units SET state = 'queued', host = NULL WHERE state = 'leased' AND lease_until < ?")
        .bind(t)
        .execute(&s.db)
        .await?;
    let rows = sqlx::query(
        "SELECT u.id, u.unit FROM units u JOIN rounds r ON r.id = u.round WHERE u.state = 'queued' \
         AND NOT EXISTS (SELECT 1 FROM failures f WHERE f.unit = u.id AND f.host = ?) \
         ORDER BY r.priority DESC, r.id, u.id LIMIT 200",
    )
    .bind(req.host)
    .fetch_all(&s.db)
    .await?;
    for row in rows {
        let mut unit: WorkUnit =
            serde_json::from_str(row.get::<String, _>("unit").as_str()).map_err(|e| bad(e.to_string()))?;
        if unit.pins.strict != strict || !(unit.pins.builds.is_empty() || unit.pins.builds.contains(&build)) {
            continue;
        }
        let until = t + LEASE_SECS as i64;
        unit.deadline = until as u64;
        sqlx::query(
            "UPDATE units SET state = 'leased', host = ?, lease_until = ?, attempts = attempts + 1 WHERE id = ?",
        )
        .bind(req.host)
        .bind(until)
        .bind(&unit.id)
        .execute(&s.db)
        .await?;
        sqlx::query("DELETE FROM uploads WHERE unit = ?").bind(&unit.id).execute(&s.db).await?;
        return Ok(Json(Lease { unit, lease_secs: LEASE_SECS }).into_response());
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// The unit, if it's leased to one of the caller's hosts: (unit, round id)
async fn leased(s: &Swarm, c: &Caller, id: &str) -> Res<(WorkUnit, i64, i64)> {
    let row = sqlx::query(
        "SELECT u.unit, u.round, u.host FROM units u JOIN hosts h ON h.id = u.host \
         WHERE u.id = ? AND u.state = 'leased' AND h.token = ?",
    )
    .bind(id)
    .bind(c.token)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(|| conflict(format!("{id} isn't leased to you (it may have expired and gone to another host)")))?;
    let unit = serde_json::from_str(row.get::<String, _>("unit").as_str()).map_err(|e| bad(e.to_string()))?;
    Ok((unit, row.get("round"), row.get("host")))
}

async fn heartbeat(State(s): State<Shared>, headers: HeaderMap, Path(id): Path<String>) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    let (_, _, host) = leased(&s, &c, &id).await?;
    sqlx::query("UPDATE units SET lease_until = ? WHERE id = ?")
        .bind(now() as i64 + LEASE_SECS as i64)
        .bind(&id)
        .execute(&s.db)
        .await?;
    sqlx::query("UPDATE hosts SET seen = ? WHERE id = ?").bind(now() as i64).bind(host).execute(&s.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn intake_key(round: i64, unit: &str, path: &str) -> String {
    format!("intake/{round}/{unit}/{path}")
}

async fn uploads(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UploadRequest>,
) -> Res<Json<Urls>> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    let (_, round, _) = leased(&s, &c, &id).await?;
    if req.files.is_empty() || req.files.len() > 64 {
        return Err(bad("1 to 64 files"));
    }
    let mut urls = BTreeMap::new();
    for f in &req.files {
        let key = intake_key(round, &id, &f.path);
        if f.path.contains('/') || !store::valid_key(&key) || f.size > MAX_FILE || f.sha256.len() != 64 {
            return Err(bad(format!("{}: a plain file name, under {MAX_FILE} bytes, with its sha256", f.path)));
        }
        sqlx::query("INSERT OR REPLACE INTO uploads (unit, path, size, sha256) VALUES (?, ?, ?, ?)")
            .bind(&id)
            .bind(&f.path)
            .bind(f.size as i64)
            .bind(&f.sha256)
            .execute(&s.db)
            .await?;
        urls.insert(f.path.clone(), s.store.put_url(&key, f.size, URL_SECS));
    }
    Ok(Json(Urls { urls }))
}

/// Intake: the manifest must match its lease (unit, protocol, networks, deals) and its uploads (every file, its hash
/// and its size in the store). The trainer's replay comes after.
async fn check_result(s: &Swarm, unit: &WorkUnit, round: i64, m: &ResultManifest) -> Res<()> {
    if m.protocol != PROTOCOL || m.unit != unit.id {
        return Err(bad("the manifest is for another unit or protocol"));
    }
    let want: BTreeSet<&str> = unit.nets.iter().map(|n| n.sha256.as_str()).collect();
    let got: BTreeSet<&str> = m.nets.iter().map(String::as_str).collect();
    if want != got {
        return Err(bad("the manifest's networks aren't the unit's"));
    }
    if unit.pins.strict && m.class != proto::determinism_class(true) {
        return Err(bad("a strict unit played by a build that isn't strict"));
    }
    let mut deals = BTreeSet::new();
    for d in m.games.iter().map(|g| g.deal).chain(m.failed.iter().copied()) {
        if d < unit.seed || d >= unit.seed + unit.games as u64 || !deals.insert(d) {
            return Err(bad(format!("deal {d}: outside the unit or twice")));
        }
    }
    if deals.len() != unit.games as usize {
        return Err(bad(format!("{} of the unit's {} deals", deals.len(), unit.games)));
    }
    if m.failed.len() * 10 > unit.games as usize {
        return Err(bad(format!("{} failed deals: more than a tenth", m.failed.len())));
    }
    let rows =
        sqlx::query("SELECT path, size, sha256 FROM uploads WHERE unit = ?").bind(&unit.id).fetch_all(&s.db).await?;
    let declared: BTreeMap<String, (i64, String)> =
        rows.iter().map(|r| (r.get("path"), (r.get("size"), r.get("sha256")))).collect();
    if declared.keys().collect::<Vec<_>>() != m.files.keys().collect::<Vec<_>>() {
        return Err(bad("the manifest's files aren't the ones uploaded"));
    }
    for (path, sha) in &m.files {
        let (size, declared_sha) = &declared[path];
        if sha != declared_sha {
            return Err(bad(format!("{path}: the manifest's sha256 isn't the declared one")));
        }
        match s.store.size(&intake_key(round, &unit.id, path)).await? {
            Some(n) if n as i64 == *size => {}
            other => return Err(bad(format!("{path}: {other:?} bytes in the store, {size} declared"))),
        }
    }
    Ok(())
}

async fn complete(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<Complete>,
) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    let (unit, round, _) = leased(&s, &c, &id).await?;
    check_result(&s, &unit, round, &req.manifest).await?;
    sqlx::query("UPDATE units SET state = 'complete', manifest = ?, games = ?, done_at = ? WHERE id = ?")
        .bind(serde_json::to_string(&req.manifest).unwrap())
        .bind(req.manifest.games.len() as i64)
        .bind(now() as i64)
        .bind(&id)
        .execute(&s.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn fail(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(f): Json<Fail>,
) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    let (_, _, host) = leased(&s, &c, &id).await?;
    sqlx::query("INSERT OR IGNORE INTO failures (unit, host) VALUES (?, ?)")
        .bind(&id)
        .bind(host)
        .execute(&s.db)
        .await?;
    let state = sqlx::query("SELECT attempts FROM units WHERE id = ?").bind(&id).fetch_one(&s.db).await?;
    let next = if state.get::<i64, _>("attempts") >= MAX_ATTEMPTS { "failed" } else { "queued" };
    sqlx::query("UPDATE units SET state = ?, host = NULL WHERE id = ?").bind(next).bind(&id).execute(&s.db).await?;
    s.audit(&c.name, "fail", &format!("{id}: {}", f.reason.chars().take(500).collect::<String>())).await;
    Ok(StatusCode::NO_CONTENT)
}

/// A unit handed back unplayed (the host stopped being idle): back to the queue with its attempt returned, and the
/// host may take it again later. A failure counts against the unit; a release doesn't.
async fn release(State(s): State<Shared>, headers: HeaderMap, Path(id): Path<String>) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    leased(&s, &c, &id).await?;
    sqlx::query("UPDATE units SET state = 'queued', host = NULL, attempts = MAX(attempts - 1, 0) WHERE id = ?")
        .bind(&id)
        .execute(&s.db)
        .await?;
    s.audit(&c.name, "release", &id).await;
    Ok(StatusCode::NO_CONTENT)
}

// ---- networks and the trainer

fn valid_sha(sha: &str) -> bool {
    sha.len() == 64 && sha.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

async fn get_net(State(s): State<Shared>, headers: HeaderMap, Path(sha): Path<String>) -> Res<Redirect> {
    caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    if !valid_sha(&sha) {
        return Err(bad("a network is named by its sha256"));
    }
    Ok(Redirect::temporary(&s.store.get_url(&format!("nets/{sha}.pmnet"), URL_SECS)))
}

#[derive(Deserialize)]
struct SizeQuery {
    size: u64,
}

/// An upload URL for a network (the trainer), by its sha256 and size
async fn put_net(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(sha): Path<String>,
    Query(q): Query<SizeQuery>,
) -> Res<Json<Urls>> {
    caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    if !valid_sha(&sha) || q.size > MAX_FILE {
        return Err(bad("a network is named by its sha256, and under 256 MB"));
    }
    let url = s.store.put_url(&format!("nets/{sha}.pmnet"), q.size, URL_SECS);
    Ok(Json(Urls { urls: BTreeMap::from([(format!("{sha}.pmnet"), url)]) }))
}

/// The download link a unit carries for a network
pub fn net_url(base: &str, sha: &str) -> String {
    format!("{base}/api/v1/nets/{sha}")
}

async fn new_round(State(s): State<Shared>, headers: HeaderMap, Json(mut r): Json<NewRound>) -> Res<Json<RoundStatus>> {
    let c = caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    if r.units.is_empty() {
        return Err(bad("a round with no units"));
    }
    for u in &r.units {
        proto::check_unit(u).map_err(|e| bad(format!("{}: {e}", u.id)))?;
        if u.project != r.project || u.run != r.run || u.round != r.round {
            return Err(bad(format!("{}: another round's unit", u.id)));
        }
        if !store::valid_key(&u.id) || u.id.contains('/') {
            return Err(bad(format!("{}: a unit id of [A-Za-z0-9._-]", u.id)));
        }
        for n in &u.nets {
            if !valid_sha(&n.sha256) || s.store.size(&format!("nets/{}.pmnet", n.sha256)).await?.is_none() {
                return Err(bad(format!("{}: network {} isn't uploaded", u.id, n.sha256)));
            }
        }
    }
    // the coordinator says where its networks are: the trainer may know it by another address than workers do
    for u in &mut r.units {
        for n in &mut u.nets {
            n.url = format!("{}/api/v1/nets/{}", s.base.trim_end_matches('/'), n.sha256);
        }
    }
    let mut tx = s.db.begin().await?;
    let id: i64 =
        sqlx::query("INSERT INTO rounds (project, run, round, priority, created) VALUES (?, ?, ?, ?, ?) RETURNING id")
            .bind(&r.project)
            .bind(&r.run)
            .bind(r.round as i64)
            .bind(r.priority as i64)
            .bind(now() as i64)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| conflict(format!("round {} of {}/{}: {e}", r.round, r.project, r.run)))?
            .get("id");
    for u in &r.units {
        sqlx::query("INSERT INTO units (id, round, unit, state) VALUES (?, ?, ?, 'queued')")
            .bind(&u.id)
            .bind(id)
            .bind(serde_json::to_string(u).unwrap())
            .execute(&mut *tx)
            .await
            .map_err(|e| conflict(format!("unit {}: {e}", u.id)))?;
    }
    tx.commit().await?;
    s.audit(&c.name, "round", &format!("{}/{} round {}: {} units", r.project, r.run, r.round, r.units.len())).await;
    round_json(&s, id).await.map(Json)
}

async fn round_json(s: &Swarm, id: i64) -> Res<RoundStatus> {
    let r = sqlx::query("SELECT project, run, round FROM rounds WHERE id = ?")
        .bind(id)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no round {id}")))?;
    let mut st = RoundStatus {
        id,
        project: r.get("project"),
        run: r.get("run"),
        round: r.get::<i64, _>("round") as u32,
        units: BTreeMap::new(),
        games: 0,
    };
    for row in sqlx::query(
        "SELECT state, COUNT(*) AS n, COALESCE(SUM(games), 0) AS g FROM units WHERE round = ? GROUP BY state",
    )
    .bind(id)
    .fetch_all(&s.db)
    .await?
    {
        let state: String = row.get("state");
        if state == "complete" || state == "accepted" {
            st.games += row.get::<i64, _>("g") as u64;
        }
        st.units.insert(state, row.get::<i64, _>("n") as u32);
    }
    Ok(st)
}

async fn round_status(State(s): State<Shared>, headers: HeaderMap, Path(id): Path<i64>) -> Res<Json<RoundStatus>> {
    caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    round_json(&s, id).await.map(Json)
}

#[derive(Deserialize)]
struct StateQuery {
    state: Option<String>,
}

async fn round_results(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Query(q): Query<StateQuery>,
) -> Res<Json<Vec<UnitResult>>> {
    caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    let state = q.state.unwrap_or_else(|| "complete".into());
    let rows = sqlx::query(
        "SELECT u.id, u.unit, u.state, u.manifest, h.name AS host, t.name AS account FROM units u \
         JOIN hosts h ON h.id = u.host JOIN tokens t ON t.id = h.token \
         WHERE u.round = ? AND u.state = ? AND u.manifest IS NOT NULL ORDER BY u.id",
    )
    .bind(id)
    .bind(&state)
    .fetch_all(&s.db)
    .await?;
    let mut out = Vec::new();
    for r in rows {
        let unit: String = r.get("id");
        let manifest: ResultManifest =
            serde_json::from_str(r.get::<String, _>("manifest").as_str()).map_err(|e| bad(e.to_string()))?;
        let files =
            manifest.files.keys().map(|p| (p.clone(), s.store.get_url(&intake_key(id, &unit, p), URL_SECS))).collect();
        let work: WorkUnit =
            serde_json::from_str(r.get::<String, _>("unit").as_str()).map_err(|e| bad(e.to_string()))?;
        out.push(UnitResult {
            unit,
            work,
            state: r.get("state"),
            host: r.get("host"),
            account: r.get("account"),
            manifest,
            files,
        });
    }
    Ok(Json(out))
}

async fn verdict(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(v): Json<Verdict>,
) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    let state = if v.accept { "accepted" } else { "rejected" };
    let n = sqlx::query("UPDATE units SET state = ?, verdict = ? WHERE id = ? AND state = 'complete'")
        .bind(state)
        .bind(&v.reason)
        .bind(&id)
        .execute(&s.db)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(conflict(format!("{id} isn't waiting for a verdict")));
    }
    s.audit(&c.name, state, &format!("{id}: {}", v.reason)).await;
    Ok(StatusCode::NO_CONTENT)
}

// ---- the local store's own routes

#[derive(Deserialize)]
struct Signed {
    exp: u64,
    size: u64,
    sig: String,
}

async fn store_put(
    State(s): State<Shared>,
    Path(key): Path<String>,
    Query(q): Query<Signed>,
    body: Bytes,
) -> Res<StatusCode> {
    let Store::Local(l) = &s.store else { return Err(ApiError(StatusCode::NOT_FOUND, "no local store".into())) };
    if !l.check("PUT", &key, q.size, q.exp, &q.sig) {
        return Err(ApiError(StatusCode::FORBIDDEN, "a live signature for this key is needed".into()));
    }
    if body.len() as u64 != q.size {
        return Err(bad(format!("{} bytes, {} signed", body.len(), q.size)));
    }
    let path = l.path(&key)?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(anyhow::Error::from)?;
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, &body).map_err(anyhow::Error::from)?;
    std::fs::rename(&tmp, &path).map_err(anyhow::Error::from)?;
    Ok(StatusCode::OK)
}

async fn store_get(State(s): State<Shared>, Path(key): Path<String>, Query(q): Query<Signed>) -> Res<Response> {
    let Store::Local(l) = &s.store else { return Err(ApiError(StatusCode::NOT_FOUND, "no local store".into())) };
    if !l.check("GET", &key, q.size, q.exp, &q.sig) {
        return Err(ApiError(StatusCode::FORBIDDEN, "a live signature for this key is needed".into()));
    }
    match std::fs::read(l.path(&key)?) {
        Ok(b) => Ok(b.into_response()),
        Err(_) => Err(ApiError(StatusCode::NOT_FOUND, format!("no {key}"))),
    }
}
