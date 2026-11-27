//! Mana@Home's coordinator (docs/swarm/Design.md): leases work units to workers, presigns their uploads, checks a
//! result against its lease (intake), schedules replay and re-run `verify` units on reputable/trusted workers,
//! tracks per-worker statistics, reputation, quarantine, credits and the admin verification inbox. It keeps metadata
//! only, in SQLite: game data goes from the worker to the store and from the store to verifiers and the trainer.
//! The API is `playmat_swarm_proto::api`.

mod admin;
mod bridge;
pub mod gates;
mod public;
pub mod store;
pub mod trust;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::header::{ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use playmat_swarm_proto::api::*;
use playmat_swarm_proto::{self as proto, ResultManifest, UnitKind, VerifyMode, VerifySpec, WorkUnit, PROTOCOL};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::collections::{BTreeMap, BTreeSet, HashMap};
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
    pub(crate) lease: tokio::sync::Mutex<()>,
    /// sliding-window registration timestamps per client IP (rate limiting `POST /api/v1/register`)
    pub(crate) reg_limits: tokio::sync::Mutex<HashMap<String, Vec<i64>>>,
}

pub type Shared = Arc<Swarm>;

const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS tokens (id INTEGER PRIMARY KEY, hash TEXT NOT NULL UNIQUE, role TEXT NOT NULL, \
     name TEXT NOT NULL, created INTEGER NOT NULL, revoked INTEGER NOT NULL DEFAULT 0, \
     trust TEXT NOT NULL DEFAULT 'probation', accepted_units INTEGER NOT NULL DEFAULT 0, \
     mismatches INTEGER NOT NULL DEFAULT 0, extra_audit_until INTEGER NOT NULL DEFAULT 0, \
     credit_frozen_until INTEGER NOT NULL DEFAULT 0)",
    "CREATE TABLE IF NOT EXISTS hosts (id INTEGER PRIMARY KEY, token INTEGER NOT NULL, name TEXT NOT NULL, \
     build TEXT NOT NULL, class TEXT NOT NULL, strict INTEGER NOT NULL, threads INTEGER NOT NULL, os TEXT NOT NULL, \
     seen INTEGER NOT NULL, accepted_units INTEGER NOT NULL DEFAULT 0, mismatches INTEGER NOT NULL DEFAULT 0, \
     probation_passed INTEGER NOT NULL DEFAULT 0, UNIQUE(token, name))",
    "CREATE TABLE IF NOT EXISTS rounds (id INTEGER PRIMARY KEY, project TEXT NOT NULL, run TEXT NOT NULL, \
     round INTEGER NOT NULL, priority INTEGER NOT NULL, created INTEGER NOT NULL, \
     trained INTEGER NOT NULL DEFAULT 0, UNIQUE(project, run, round))",
    "CREATE TABLE IF NOT EXISTS units (id TEXT PRIMARY KEY, round INTEGER NOT NULL, unit TEXT NOT NULL, \
     state TEXT NOT NULL, host INTEGER, lease_until INTEGER, attempts INTEGER NOT NULL DEFAULT 0, manifest TEXT, \
     games INTEGER NOT NULL DEFAULT 0, done_at INTEGER, verdict TEXT, \
     kind TEXT NOT NULL DEFAULT 'selfplay', source_unit TEXT, verify_mode TEXT, source_host INTEGER, \
     source_token INTEGER, leased_at INTEGER, credit REAL NOT NULL DEFAULT 0.0, \
     credit_status TEXT NOT NULL DEFAULT 'none', credit_mature_at INTEGER, quarantined INTEGER NOT NULL DEFAULT 0)",
    "CREATE INDEX IF NOT EXISTS units_state ON units(state, round)",
    "CREATE TABLE IF NOT EXISTS uploads (unit TEXT NOT NULL, path TEXT NOT NULL, size INTEGER NOT NULL, \
     sha256 TEXT NOT NULL, PRIMARY KEY(unit, path))",
    // a host that gave a unit back never gets it again: a broken host can't use up a unit's attempts alone
    "CREATE TABLE IF NOT EXISTS failures (unit TEXT NOT NULL, host INTEGER NOT NULL, PRIMARY KEY(unit, host))",
    "CREATE TABLE IF NOT EXISTS audit (id INTEGER PRIMARY KEY, at INTEGER NOT NULL, actor TEXT NOT NULL, \
     action TEXT NOT NULL, detail TEXT NOT NULL)",
    "CREATE TABLE IF NOT EXISTS unit_stats (unit TEXT PRIMARY KEY, round INTEGER NOT NULL, token INTEGER NOT NULL, \
     host INTEGER NOT NULL, kind TEXT NOT NULL, games INTEGER NOT NULL, learner_wins INTEGER NOT NULL, \
     draws INTEGER NOT NULL, total_turns INTEGER NOT NULL, total_answers INTEGER NOT NULL, \
     total_recorded INTEGER NOT NULL, mean_entropy REAL, ms_per_game REAL NOT NULL, created INTEGER NOT NULL)",
    "CREATE INDEX IF NOT EXISTS unit_stats_token ON unit_stats(token, kind, created)",
    "CREATE TABLE IF NOT EXISTS inbox (id INTEGER PRIMARY KEY, created INTEGER NOT NULL, status TEXT NOT NULL, \
     kind TEXT NOT NULL, token INTEGER, host INTEGER, unit TEXT, verify_unit TEXT, detail TEXT NOT NULL, \
     resolved_by TEXT, resolved_at INTEGER, resolution TEXT)",
    "CREATE INDEX IF NOT EXISTS inbox_status ON inbox(status, id)",
    "CREATE TABLE IF NOT EXISTS credits (id INTEGER PRIMARY KEY, at INTEGER NOT NULL, token INTEGER NOT NULL, \
     host INTEGER, unit TEXT, kind TEXT NOT NULL, base_credit REAL NOT NULL, bonus_mult REAL NOT NULL, \
     credit REAL NOT NULL, status TEXT NOT NULL, mature_at INTEGER NOT NULL, reason TEXT NOT NULL DEFAULT '')",
    "CREATE INDEX IF NOT EXISTS credits_token ON credits(token, status, at)",
    "CREATE TABLE IF NOT EXISTS teams (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, \
     description TEXT NOT NULL DEFAULT '', created INTEGER NOT NULL, owner_token INTEGER NOT NULL)",
    "CREATE TABLE IF NOT EXISTS networks (sha256 TEXT PRIMARY KEY, label TEXT NOT NULL, parent_sha TEXT, \
     status TEXT NOT NULL, created INTEGER NOT NULL, promoted_at INTEGER, bench_elo REAL, bench_ci95 REAL, \
     bench_games INTEGER NOT NULL DEFAULT 0, anchors_json TEXT NOT NULL DEFAULT '[]')",
    "CREATE TABLE IF NOT EXISTS gates (id INTEGER PRIMARY KEY, round_id INTEGER NOT NULL UNIQUE, \
     candidate_sha TEXT NOT NULL, candidate_label TEXT NOT NULL, baseline_sha TEXT NOT NULL, \
     baseline_label TEXT NOT NULL, status TEXT NOT NULL, held INTEGER NOT NULL DEFAULT 0, \
     auto_promote INTEGER NOT NULL DEFAULT 1, elo0 REAL NOT NULL, elo1 REAL NOT NULL, alpha REAL NOT NULL, \
     beta REAL NOT NULL, max_share REAL NOT NULL, wins INTEGER NOT NULL DEFAULT 0, draws INTEGER NOT NULL DEFAULT 0, \
     losses INTEGER NOT NULL DEFAULT 0, elo REAL NOT NULL DEFAULT 0.0, elo_ci95 REAL NOT NULL DEFAULT 0.0, \
     llr REAL NOT NULL DEFAULT 0.0, top_account_share REAL NOT NULL DEFAULT 0.0, created INTEGER NOT NULL, \
     decided_at INTEGER, decision_reason TEXT NOT NULL DEFAULT '')",
    "CREATE TABLE IF NOT EXISTS benches (id INTEGER PRIMARY KEY, round_id INTEGER NOT NULL UNIQUE, \
     net_sha TEXT NOT NULL, net_label TEXT NOT NULL, anchor_elo REAL NOT NULL, games INTEGER NOT NULL DEFAULT 0, \
     elo REAL NOT NULL DEFAULT 1000.0, ci95 REAL NOT NULL DEFAULT 0.0, anchors_json TEXT NOT NULL DEFAULT '[]', \
     created INTEGER NOT NULL, updated_at INTEGER NOT NULL)",
    "CREATE TABLE IF NOT EXISTS credit_redemptions (id INTEGER PRIMARY KEY, token INTEGER NOT NULL, \
     playmat_user TEXT NOT NULL, credit_spent REAL NOT NULL, capsule_tokens INTEGER NOT NULL, created INTEGER NOT NULL)",
];

/// Idempotent S1 -> S2/S3 column additions for existing SQLite databases.
const MIGRATIONS: &[&str] = &[
    "ALTER TABLE tokens ADD COLUMN trust TEXT NOT NULL DEFAULT 'probation'",
    "ALTER TABLE tokens ADD COLUMN accepted_units INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE tokens ADD COLUMN mismatches INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE tokens ADD COLUMN extra_audit_until INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE tokens ADD COLUMN credit_frozen_until INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE tokens ADD COLUMN team_id INTEGER",
    "ALTER TABLE tokens ADD COLUMN team_joined_at INTEGER",
    "ALTER TABLE hosts ADD COLUMN accepted_units INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE hosts ADD COLUMN mismatches INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE hosts ADD COLUMN probation_passed INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE rounds ADD COLUMN trained INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE units ADD COLUMN kind TEXT NOT NULL DEFAULT 'selfplay'",
    "ALTER TABLE units ADD COLUMN source_unit TEXT",
    "ALTER TABLE units ADD COLUMN verify_mode TEXT",
    "ALTER TABLE units ADD COLUMN source_host INTEGER",
    "ALTER TABLE units ADD COLUMN source_token INTEGER",
    "ALTER TABLE units ADD COLUMN leased_at INTEGER",
    "ALTER TABLE units ADD COLUMN credit REAL NOT NULL DEFAULT 0.0",
    "ALTER TABLE units ADD COLUMN credit_status TEXT NOT NULL DEFAULT 'none'",
    "ALTER TABLE units ADD COLUMN credit_mature_at INTEGER",
    "ALTER TABLE units ADD COLUMN quarantined INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE hosts ADD COLUMN gpu TEXT",
    "ALTER TABLE rounds ADD COLUMN archived INTEGER NOT NULL DEFAULT 0",
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
        for m in MIGRATIONS {
            if let Err(e) = sqlx::query(m).execute(&db).await {
                if !e.to_string().contains("duplicate column name") {
                    return Err(e.into());
                }
            }
        }
        sqlx::query("CREATE INDEX IF NOT EXISTS units_source ON units(source_unit)").execute(&db).await?;
        sqlx::query("UPDATE tokens SET trust = 'trusted' WHERE role IN ('trainer', 'admin') AND trust = 'probation'")
            .execute(&db)
            .await?;
        Ok(Arc::new(Swarm {
            db,
            store,
            base: base.trim_end_matches('/').to_string(),
            lease: tokio::sync::Mutex::new(()),
            reg_limits: tokio::sync::Mutex::new(HashMap::new()),
        }))
    }

    /// Makes a token with default trust (`Probation` for worker, `Trusted` for trainer/admin)
    pub async fn new_token(&self, role: Role, name: &str) -> anyhow::Result<TokenReply> {
        self.new_token_with_trust(role, name, None).await
    }

    /// Makes a token with an explicit trust tier and keeps only its hash; the token itself is shown once
    pub async fn new_token_with_trust(
        &self,
        role: Role,
        name: &str,
        trust: Option<TrustLevel>,
    ) -> anyhow::Result<TokenReply> {
        let trust = trust.unwrap_or(match role {
            Role::Worker => TrustLevel::Probation,
            Role::Trainer | Role::Admin => TrustLevel::Trusted,
        });
        let mut raw = [0u8; 24];
        getrandom::getrandom(&mut raw).map_err(|e| anyhow::anyhow!("randomness: {e}"))?;
        let token = format!("mana_{}_{}", role.name(), hex(&raw));
        let id = sqlx::query("INSERT INTO tokens (hash, role, name, created, trust) VALUES (?, ?, ?, ?, ?)")
            .bind(hash(&token))
            .bind(role.name())
            .bind(name)
            .bind(now() as i64)
            .bind(trust.name())
            .execute(&self.db)
            .await?
            .last_insert_rowid();
        self.audit("cli", "token", &format!("{} token {id} for {name} (trust={})", role.name(), trust.name())).await;
        Ok(TokenReply { id, token })
    }

    pub(crate) async fn audit(&self, actor: &str, action: &str, detail: &str) {
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

async fn cors(req: Request, next: Next) -> Response {
    let mut res =
        if req.method() == Method::OPTIONS { StatusCode::NO_CONTENT.into_response() } else { next.run(req).await };
    let h = res.headers_mut();
    h.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    h.insert(ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, POST, PUT, OPTIONS"));
    h.insert(ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("Authorization, Content-Type"));
    res
}

async fn root_redirect() -> Redirect {
    let target = std::env::var("SWARM_TRACKER_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "https://playmat.deimos.studio/swarm".to_string());
    Redirect::temporary(&target)
}

pub fn router(s: Shared) -> Router {
    Router::new()
        .route("/", get(root_redirect))
        .route("/api/v1/status", get(public::status))
        .route("/api/v1/events", get(public::events))
        .route("/api/v1/projects", get(public::projects))
        .route("/api/v1/feed", get(public::feed))
        .route("/api/v1/leaderboard", get(public::leaderboard))
        .route("/api/v1/teams", get(public::list_teams).post(public::create_team))
        .route("/api/v1/teams/{id}/join", post(public::join_team))
        .route("/api/v1/teams/leave", post(public::leave_team))
        .route("/api/v1/contributors/{name}", get(public::contributor_profile))
        .route("/api/v1/transparency", get(public::transparency))
        .route("/api/v1/register", post(public::register))
        .route("/api/v1/watch", get(public::watch_list))
        .route("/api/v1/watch/{unit}/{deal}", get(public::watch_replay))
        .route("/api/v1/credits/balance", get(bridge::balance))
        .route("/api/v1/bridge/redeem", post(bridge::redeem))
        .route("/api/v1/networks", get(gates::list_networks))
        .route("/api/v1/gates", get(gates::list_gates).post(gates::new_gate))
        .route("/api/v1/gates/{id}", get(gates::get_gate))
        .route("/api/v1/benches", get(gates::list_benches))
        .route("/api/v1/bench", post(gates::new_bench))
        .route("/api/v1/tokens", post(new_token))
        .route("/api/v1/hello", post(hello))
        .route("/api/v1/lease", post(lease))
        .route("/api/v1/units/{id}/heartbeat", post(heartbeat))
        .route("/api/v1/units/{id}/uploads", post(uploads))
        .route("/api/v1/units/{id}/complete", post(complete))
        .route("/api/v1/units/{id}/verify-complete", post(verify_complete))
        .route("/api/v1/units/{id}/fail", post(fail))
        .route("/api/v1/units/{id}/release", post(release))
        .route("/api/v1/units/{id}/cancel", post(cancel_unit))
        .route("/api/v1/units/{id}/verdict", post(verdict))
        .route("/api/v1/nets/{sha}", get(get_net).post(put_net))
        .route("/api/v1/rounds", get(list_rounds).post(new_round))
        .route("/api/v1/rounds/{id}", get(round_status))
        .route("/api/v1/rounds/{id}/priority", post(set_round_priority))
        .route("/api/v1/rounds/{id}/cancel", post(cancel_round))
        .route("/api/v1/rounds/{id}/results", get(round_results))
        .route("/api/v1/admin/workers", get(admin::list_workers))
        .route("/api/v1/admin/tokens/{id}/trust", post(admin::set_token_trust))
        .route("/api/v1/admin/tokens/{id}/revoke", post(admin::revoke_token))
        .route("/api/v1/admin/tokens/{id}/quarantine", post(admin::quarantine_token))
        .route("/api/v1/admin/tokens/{id}/restore", post(admin::restore_token))
        .route("/api/v1/admin/tokens/{id}/credit", post(admin::adjust_credit))
        .route("/api/v1/admin/rounds/{id}/trained", post(admin::mark_round_trained))
        .route("/api/v1/admin/rounds/{id}/purge-intake", post(admin::purge_round_intake))
        .route("/api/v1/admin/units/{id}/rerun", post(admin::order_unit_rerun))
        .route("/api/v1/admin/units/{id}/reissue", post(admin::reissue_unit))
        .route("/api/v1/admin/inbox", get(admin::list_inbox))
        .route("/api/v1/admin/inbox/{id}/resolve", post(admin::resolve_inbox))
        .route("/api/v1/admin/gates/{id}/hold", post(gates::hold_gate))
        .route("/api/v1/admin/gates/{id}/resume", post(gates::resume_gate))
        .route("/api/v1/admin/gates/{id}/promote", post(gates::promote_gate))
        .route("/api/v1/admin/networks/rollback", post(gates::rollback_network))
        .route("/api/v1/admin/audit", get(admin::list_audit))
        .route("/store/{*key}", get(store_get).put(store_put).layer(DefaultBodyLimit::max(MAX_FILE as usize)))
        .layer(middleware::from_fn(cors))
        .with_state(s)
}

// ---- errors and auth

pub struct ApiError(pub(crate) StatusCode, pub(crate) String);

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

pub(crate) fn bad(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

pub(crate) fn conflict(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::CONFLICT, msg.into())
}

pub(crate) type Res<T> = Result<T, ApiError>;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hash(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
}

/// Who called: the token's id, name, and trust tier
pub(crate) struct Caller {
    pub(crate) token: i64,
    pub(crate) name: String,
    pub(crate) trust: TrustLevel,
}

pub(crate) async fn caller(s: &Swarm, headers: &HeaderMap, roles: &[Role]) -> Res<Caller> {
    let unauthorized = || ApiError(StatusCode::UNAUTHORIZED, "a valid bearer token is needed".into());
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(unauthorized)?;
    let row = sqlx::query("SELECT id, role, name, trust FROM tokens WHERE hash = ? AND revoked = 0")
        .bind(hash(token))
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(unauthorized)?;
    let role = Role::parse(row.get::<String, _>("role").as_str()).ok_or_else(unauthorized)?;
    if !roles.contains(&role) {
        return Err(ApiError(StatusCode::FORBIDDEN, format!("a {} token can't do that", role.name())));
    }
    let trust = TrustLevel::parse(row.get::<String, _>("trust").as_str()).unwrap_or(TrustLevel::Probation);
    Ok(Caller { token: row.get("id"), name: row.get("name"), trust })
}

pub(crate) fn require_not_banned(c: &Caller) -> Res<()> {
    if c.trust == TrustLevel::Banned {
        return Err(ApiError(StatusCode::FORBIDDEN, "account is banned".into()));
    }
    Ok(())
}

// ---- admin

async fn new_token(State(s): State<Shared>, headers: HeaderMap, Json(req): Json<NewToken>) -> Res<Json<TokenReply>> {
    let c = caller(&s, &headers, &[Role::Admin]).await?;
    let reply = s.new_token_with_trust(req.role, &req.name, req.trust).await?;
    s.audit(
        &c.name,
        "token",
        &format!("{} token {} for {} (trust={:?})", req.role.name(), reply.id, req.name, req.trust),
    )
    .await;
    Ok(Json(reply))
}

// ---- worker

async fn hello(State(s): State<Shared>, headers: HeaderMap, Json(h): Json<Hello>) -> Res<Json<HelloReply>> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    require_not_banned(&c)?;
    if h.name.is_empty() || h.name.len() > 64 {
        return Err(bad("a host name of 1 to 64 characters"));
    }
    let probation_passed = i64::from(c.trust == TrustLevel::Trusted);
    let gpu = h.gpu.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    let id: i64 = sqlx::query(
        "INSERT INTO hosts (token, name, build, class, strict, threads, os, gpu, seen, probation_passed) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(token, name) DO UPDATE SET build = excluded.build, class = excluded.class, \
         strict = excluded.strict, threads = excluded.threads, os = excluded.os, gpu = excluded.gpu, \
         seen = excluded.seen RETURNING id",
    )
    .bind(c.token)
    .bind(&h.name)
    .bind(&h.build)
    .bind(&h.class)
    .bind(h.strict)
    .bind(h.threads as i64)
    .bind(&h.os)
    .bind(&gpu)
    .bind(now() as i64)
    .bind(probation_passed)
    .fetch_one(&s.db)
    .await?
    .get("id");
    Ok(Json(HelloReply { host: id, poll_secs: POLL_SECS }))
}

/// A host of the caller's token: (build, strict, gpu)
async fn own_host(s: &Swarm, c: &Caller, host: i64) -> Res<(String, bool, Option<String>)> {
    let row = sqlx::query("SELECT build, strict, gpu FROM hosts WHERE id = ? AND token = ?")
        .bind(host)
        .bind(c.token)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| bad("no such host on this token: say hello first"))?;
    sqlx::query("UPDATE hosts SET seen = ? WHERE id = ?").bind(now() as i64).bind(host).execute(&s.db).await?;
    Ok((row.get("build"), row.get::<i64, _>("strict") != 0, row.get("gpu")))
}

async fn lease(State(s): State<Shared>, headers: HeaderMap, Json(req): Json<LeaseRequest>) -> Res<Response> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    require_not_banned(&c)?;
    let (build, strict, gpu) = own_host(&s, &c, req.host).await?;
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
    // verify units go first for eligible verifiers, then by round priority
    let rows = sqlx::query(
        "SELECT u.id, u.round, u.unit, u.kind, u.source_unit, u.verify_mode, u.source_token \
         FROM units u JOIN rounds r ON r.id = u.round WHERE u.state = 'queued' \
         AND NOT EXISTS (SELECT 1 FROM failures f WHERE f.unit = u.id AND f.host = ?) \
         ORDER BY CASE WHEN u.kind = 'verify' THEN 1 ELSE 0 END DESC, r.priority DESC, r.id, u.id LIMIT 200",
    )
    .bind(req.host)
    .fetch_all(&s.db)
    .await?;
    for row in rows {
        let kind_s: String = row.get("kind");
        let kind = trust::unit_kind_parse(&kind_s);
        let source_token: Option<i64> = row.get("source_token");
        let verify_mode_s: Option<String> = row.get("verify_mode");

        match kind {
            UnitKind::Verify => {
                // only reputable/trusted workers may verify, and never the same account that played the unit
                if !c.trust.can_verify() || source_token == Some(c.token) {
                    continue;
                }
            }
            UnitKind::Match | UnitKind::Bench => {
                if !c.trust.can_gate() {
                    continue;
                }
            }
            UnitKind::Selfplay => {}
            UnitKind::SelfplayGpu => {
                if gpu.is_none() {
                    continue;
                }
            }
        }

        let mut unit: WorkUnit =
            serde_json::from_str(row.get::<String, _>("unit").as_str()).map_err(|e| bad(e.to_string()))?;
        let needs_strict_match =
            kind != UnitKind::SelfplayGpu && !(kind == UnitKind::Verify && verify_mode_s.as_deref() == Some("replay"));
        if (needs_strict_match && unit.pins.strict != strict)
            || !(unit.pins.builds.is_empty() || unit.pins.builds.contains(&build))
        {
            continue;
        }

        // populate fresh presigned GET URLs for verify units so URLs never expire while queued
        if kind == UnitKind::Verify {
            let round: i64 = row.get("round");
            if let Some(ref mut spec) = unit.verify {
                let target = spec.target_unit.clone();
                let urows = sqlx::query("SELECT path FROM uploads WHERE unit = ? ORDER BY path")
                    .bind(&target)
                    .fetch_all(&s.db)
                    .await?;
                spec.files.clear();
                for ur in urows {
                    let path: String = ur.get("path");
                    spec.files.insert(path.clone(), s.store.get_url(&intake_key(round, &target, &path), URL_SECS));
                }
            }
        }

        let until = t + LEASE_SECS as i64;
        unit.deadline = until as u64;
        sqlx::query(
            "UPDATE units SET state = 'leased', host = ?, lease_until = ?, leased_at = ?, attempts = attempts + 1 \
             WHERE id = ?",
        )
        .bind(req.host)
        .bind(until)
        .bind(t)
        .bind(&unit.id)
        .execute(&s.db)
        .await?;
        sqlx::query("DELETE FROM uploads WHERE unit = ?").bind(&unit.id).execute(&s.db).await?;
        return Ok(Json(Lease { unit, lease_secs: LEASE_SECS }).into_response());
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// The unit, if it's leased to one of the caller's hosts: (unit, round id, host id, leased_at)
async fn leased(s: &Swarm, c: &Caller, id: &str) -> Res<(WorkUnit, i64, i64, Option<i64>)> {
    let row = sqlx::query(
        "SELECT u.unit, u.round, u.host, u.leased_at FROM units u JOIN hosts h ON h.id = u.host \
         WHERE u.id = ? AND u.state = 'leased' AND h.token = ?",
    )
    .bind(id)
    .bind(c.token)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(|| conflict(format!("{id} isn't leased to you (it may have expired and gone to another host)")))?;
    let unit = serde_json::from_str(row.get::<String, _>("unit").as_str()).map_err(|e| bad(e.to_string()))?;
    Ok((unit, row.get("round"), row.get("host"), row.get("leased_at")))
}

async fn heartbeat(State(s): State<Shared>, headers: HeaderMap, Path(id): Path<String>) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    require_not_banned(&c)?;
    let (_, _, host, _) = leased(&s, &c, &id).await?;
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
    require_not_banned(&c)?;
    let (_, round, _, _) = leased(&s, &c, &id).await?;
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
/// and its size in the store). Replay and re-run verification come next.
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

/// Spawns a `UnitKind::Verify` unit (`Replay` or `Rerun`) for a completed/accepted target unit.
pub(crate) async fn spawn_verify_unit(s: &Swarm, target_id: &str, mode: VerifyMode, extra: bool) -> Res<String> {
    let row = sqlx::query(
        "SELECT u.round, u.unit, u.manifest, u.host, h.token AS source_token \
         FROM units u JOIN hosts h ON h.id = u.host \
         WHERE u.id = ? AND u.kind != 'verify' AND u.manifest IS NOT NULL",
    )
    .bind(target_id)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(|| bad(format!("{target_id}: no completed unit to verify")))?;

    let round: i64 = row.get("round");
    let source_host: i64 = row.get("host");
    let source_token: i64 = row.get("source_token");
    let target_work: WorkUnit =
        serde_json::from_str(row.get::<String, _>("unit").as_str()).map_err(|e| bad(e.to_string()))?;
    let manifest: ResultManifest =
        serde_json::from_str(row.get::<String, _>("manifest").as_str()).map_err(|e| bad(e.to_string()))?;

    let mode_tag = match mode {
        VerifyMode::Replay => "replay",
        VerifyMode::Rerun => "rerun",
    };
    let mut vid = format!("v-{mode_tag}-{target_id}");
    if extra {
        let mut suffix = 1u32;
        loop {
            let candidate = format!("v-{mode_tag}-{target_id}-x{suffix}");
            let exists: i64 = sqlx::query("SELECT COUNT(*) AS n FROM units WHERE id = ?")
                .bind(&candidate)
                .fetch_one(&s.db)
                .await?
                .get("n");
            if exists == 0 {
                vid = candidate;
                break;
            }
            suffix += 1;
        }
    } else {
        let exists: i64 =
            sqlx::query("SELECT COUNT(*) AS n FROM units WHERE id = ?").bind(&vid).fetch_one(&s.db).await?.get("n");
        if exists > 0 {
            let mut suffix = 2u32;
            loop {
                let candidate = format!("v-{mode_tag}-{target_id}-{suffix}");
                let e: i64 = sqlx::query("SELECT COUNT(*) AS n FROM units WHERE id = ?")
                    .bind(&candidate)
                    .fetch_one(&s.db)
                    .await?
                    .get("n");
                if e == 0 {
                    vid = candidate;
                    break;
                }
                suffix += 1;
            }
        }
    }

    let cost = match mode {
        VerifyMode::Rerun => target_work.cost,
        VerifyMode::Replay => (target_work.cost * 0.05).max(0.1),
    };
    let mut verify_work = target_work.clone();
    verify_work.id = vid.clone();
    verify_work.kind = UnitKind::Verify;
    verify_work.cost = cost;
    verify_work.verify = Some(VerifySpec {
        mode,
        target_unit: target_id.into(),
        target_kind: target_work.kind,
        manifest,
        files: BTreeMap::new(),
    });

    sqlx::query(
        "INSERT INTO units (id, round, unit, state, kind, source_unit, verify_mode, source_host, source_token) \
         VALUES (?, ?, ?, 'queued', 'verify', ?, ?, ?, ?)",
    )
    .bind(&vid)
    .bind(round)
    .bind(serde_json::to_string(&verify_work).unwrap())
    .bind(target_id)
    .bind(mode_tag)
    .bind(source_host)
    .bind(source_token)
    .execute(&s.db)
    .await?;
    Ok(vid)
}

async fn complete(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<Complete>,
) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    require_not_banned(&c)?;
    let (unit, round, host, _) = leased(&s, &c, &id).await?;
    if unit.kind == UnitKind::Verify {
        return Err(bad("verify units report to /verify-complete"));
    }
    check_result(&s, &unit, round, &req.manifest).await?;
    sqlx::query("UPDATE units SET state = 'complete', manifest = ?, games = ?, done_at = ? WHERE id = ?")
        .bind(serde_json::to_string(&req.manifest).unwrap())
        .bind(req.manifest.games.len() as i64)
        .bind(now() as i64)
        .bind(&id)
        .execute(&s.db)
        .await?;

    // S2: record per-worker stats and check residuals against peers
    trust::record_unit_stats(&s.db, &id, round, c.token, host, unit.kind, &req.manifest).await?;
    if let Some(flag) = trust::check_worker_stats(&s.db, round, c.token, host, unit.kind, &id).await? {
        s.audit("coordinator", "stat-flag", &format!("{}/{id}: {}", c.name, flag.detail)).await;
    }

    // S2: schedule replay or re-run verification based on trust tier, host probation, and statistical flags
    let trow =
        sqlx::query("SELECT trust, extra_audit_until FROM tokens WHERE id = ?").bind(c.token).fetch_one(&s.db).await?;
    let current_trust = TrustLevel::parse(trow.get::<String, _>("trust").as_str()).unwrap_or(TrustLevel::Probation);
    let flagged = trow.get::<i64, _>("extra_audit_until") > now() as i64;
    let hrow = sqlx::query("SELECT probation_passed FROM hosts WHERE id = ?").bind(host).fetch_one(&s.db).await?;
    let host_probation_passed = hrow.get::<i64, _>("probation_passed") != 0;

    let mode = if trust::should_rerun(current_trust, host_probation_passed, flagged, unit.kind, trust::random_roll()) {
        VerifyMode::Rerun
    } else {
        VerifyMode::Replay
    };
    let _ = spawn_verify_unit(&s, &id, mode, false).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn verify_complete(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<VerifyReport>,
) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    require_not_banned(&c)?;
    let (unit, _, verifier_host, leased_at) = leased(&s, &c, &id).await?;
    if unit.kind != UnitKind::Verify {
        return Err(bad(format!("{id} is not a verify unit")));
    }
    let Some(spec) = &unit.verify else {
        return Err(bad("verify unit missing spec"));
    };
    if spec.target_unit != req.target_unit || spec.mode != req.mode {
        return Err(bad("verify report doesn't match the leased verify unit"));
    }

    let t = now() as i64;
    sqlx::query("UPDATE units SET state = 'accepted', done_at = ?, verdict = ? WHERE id = ?")
        .bind(t)
        .bind(&req.reason)
        .bind(&id)
        .execute(&s.db)
        .await?;

    // Verifier earns full credit for the verification unit
    let ver_elapsed = leased_at.map_or(0, |la| (t - la).max(1) as u64);
    trust::award_credit(
        &s.db,
        trust::CreditGrant {
            token: c.token,
            host: Some(verifier_host),
            unit_id: &id,
            kind: "verify",
            cost: unit.cost,
            elapsed_secs: ver_elapsed,
            window_secs: LEASE_SECS,
        },
    )
    .await?;

    // Apply verdict to target unit
    let trow = sqlx::query(
        "SELECT u.round, u.state, u.unit, u.host, u.leased_at, u.done_at, h.token AS source_token \
         FROM units u JOIN hosts h ON h.id = u.host WHERE u.id = ?",
    )
    .bind(&req.target_unit)
    .fetch_optional(&s.db)
    .await?;
    let Some(trow) = trow else {
        return Ok(StatusCode::NO_CONTENT);
    };
    let target_round: i64 = trow.get("round");
    let target_state: String = trow.get("state");
    let source_host: i64 = trow.get("host");
    let source_token: i64 = trow.get("source_token");
    let target_work: WorkUnit =
        serde_json::from_str(trow.get::<String, _>("unit").as_str()).map_err(|e| bad(e.to_string()))?;

    if req.passed {
        if target_state == "complete" {
            sqlx::query("UPDATE units SET state = 'accepted', verdict = ?, quarantined = 0 WHERE id = ?")
                .bind(&req.reason)
                .bind(&req.target_unit)
                .execute(&s.db)
                .await?;
            let src_leased: Option<i64> = trow.get("leased_at");
            let src_done: Option<i64> = trow.get("done_at");
            let elapsed = match (src_leased, src_done) {
                (Some(l), Some(d)) => (d - l).max(1) as u64,
                _ => LEASE_SECS,
            };
            trust::award_credit(
                &s.db,
                trust::CreditGrant {
                    token: source_token,
                    host: Some(source_host),
                    unit_id: &req.target_unit,
                    kind: "base",
                    cost: target_work.cost,
                    elapsed_secs: elapsed,
                    window_secs: LEASE_SECS,
                },
            )
            .await?;
            if let Some(promoted) = trust::record_acceptance(&s.db, source_token, source_host, true).await? {
                s.audit("coordinator", "promote", &format!("token {source_token} -> {}", promoted.name())).await;
            }
            s.audit(&c.name, "verify-accept", &format!("{} ({:?}): {}", req.target_unit, req.mode, req.reason)).await;
        }
    } else if matches!(target_state.as_str(), "complete" | "accepted" | "quarantined") {
        let mut reason = req.reason.clone();
        if !req.mismatches.is_empty() {
            reason.push_str(&format!("; mismatched deals: {:?}", &req.mismatches[..req.mismatches.len().min(10)]));
        }
        if !req.file_mismatches.is_empty() {
            reason.push_str(&format!(
                "; mismatched files: {:?}",
                &req.file_mismatches[..req.file_mismatches.len().min(10)]
            ));
        }
        let (q_units, f_credits) =
            trust::handle_mismatch(&s.db, source_token, source_host, &req.target_unit, Some(&id), req.mode, &reason)
                .await?;
        s.audit(
            &c.name,
            "verify-reject",
            &format!("{} ({:?}): {reason} (quarantined={q_units}, frozen={f_credits})", req.target_unit, req.mode),
        )
        .await;
    }

    gates::on_round_updated(&s, target_round).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn fail(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(f): Json<Fail>,
) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    let (_, _, host, _) = leased(&s, &c, &id).await?;
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

pub(crate) fn valid_sha(sha: &str) -> bool {
    sha.len() == 64 && sha.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

async fn get_net(State(s): State<Shared>, headers: HeaderMap, Path(sha): Path<String>) -> Res<Redirect> {
    if !valid_sha(&sha) {
        return Err(bad("a network is named by its sha256"));
    }
    let is_public: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM networks WHERE sha256 = ?").bind(&sha).fetch_one(&s.db).await?;
    if is_public == 0 {
        caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
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
        if u.kind == UnitKind::Verify {
            return Err(bad(format!("{}: verify units are created by the coordinator", u.id)));
        }
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
        sqlx::query("INSERT INTO units (id, round, unit, state, kind) VALUES (?, ?, ?, 'queued', ?)")
            .bind(&u.id)
            .bind(id)
            .bind(serde_json::to_string(u).unwrap())
            .bind(trust::unit_kind_name(u.kind))
            .execute(&mut *tx)
            .await
            .map_err(|e| conflict(format!("unit {}: {e}", u.id)))?;
    }
    tx.commit().await?;
    s.audit(&c.name, "round", &format!("{}/{} round {}: {} units", r.project, r.run, r.round, r.units.len())).await;
    round_json(&s, id).await.map(Json)
}

async fn round_json(s: &Swarm, id: i64) -> Res<RoundStatus> {
    let r = sqlx::query("SELECT project, run, round, priority FROM rounds WHERE id = ?")
        .bind(id)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no round {id}")))?;
    let mut st = RoundStatus {
        id,
        project: r.get("project"),
        run: r.get("run"),
        round: r.get::<i64, _>("round") as u32,
        priority: r.get::<i64, _>("priority") as i32,
        units: BTreeMap::new(),
        games: 0,
        verify_pending: 0,
    };
    for row in sqlx::query(
        "SELECT state, COUNT(*) AS n, COALESCE(SUM(games), 0) AS g FROM units \
         WHERE round = ? AND kind != 'verify' GROUP BY state",
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
    st.verify_pending = sqlx::query(
        "SELECT COUNT(*) AS n FROM units WHERE round = ? AND kind = 'verify' AND state IN ('queued', 'leased')",
    )
    .bind(id)
    .fetch_one(&s.db)
    .await?
    .get::<i64, _>("n") as u32;
    Ok(st)
}

async fn list_rounds(State(s): State<Shared>, headers: HeaderMap) -> Res<Json<Vec<RoundStatus>>> {
    caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    let rows = sqlx::query("SELECT id FROM rounds ORDER BY id DESC").fetch_all(&s.db).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(round_json(&s, row.get("id")).await?);
    }
    Ok(Json(out))
}

async fn round_status(State(s): State<Shared>, headers: HeaderMap, Path(id): Path<i64>) -> Res<Json<RoundStatus>> {
    caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    round_json(&s, id).await.map(Json)
}

async fn set_round_priority(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<RoundPriority>,
) -> Res<Json<RoundStatus>> {
    let c = caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    let n = sqlx::query("UPDATE rounds SET priority = ? WHERE id = ?")
        .bind(req.priority as i64)
        .bind(id)
        .execute(&s.db)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, format!("no round {id}")));
    }
    s.audit(&c.name, "priority", &format!("round {id} -> {}", req.priority)).await;
    round_json(&s, id).await.map(Json)
}

async fn cancel_round(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<Cancel>,
) -> Res<Json<CancelReply>> {
    let c = caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    round_json(&s, id).await?;
    let cancelled = {
        let _one = s.lease.lock().await;
        let query = if req.include_leased {
            "UPDATE units SET state = 'cancelled', host = NULL, lease_until = NULL, verdict = ? \
             WHERE round = ? AND kind != 'verify' AND state IN ('queued', 'leased')"
        } else {
            "UPDATE units SET state = 'cancelled', host = NULL, lease_until = NULL, verdict = ? \
             WHERE round = ? AND kind != 'verify' AND state = 'queued'"
        };
        sqlx::query(query).bind(&req.reason).bind(id).execute(&s.db).await?.rows_affected() as u32
    };
    let detail = if req.reason.is_empty() {
        format!("round {id}: {cancelled} units (include_leased={})", req.include_leased)
    } else {
        format!("round {id}: {cancelled} units (include_leased={}): {}", req.include_leased, req.reason)
    };
    s.audit(&c.name, "cancel", &detail).await;
    let round = round_json(&s, id).await?;
    Ok(Json(CancelReply { cancelled, round }))
}

async fn cancel_unit(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<Cancel>,
) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    let _one = s.lease.lock().await;
    let row = sqlx::query("SELECT state FROM units WHERE id = ?")
        .bind(&id)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no unit {id}")))?;
    let state: String = row.get("state");
    if state != "queued" && state != "leased" {
        return Err(conflict(format!("{id} is {state}, can't cancel")));
    }
    sqlx::query("UPDATE units SET state = 'cancelled', host = NULL, lease_until = NULL, verdict = ? WHERE id = ?")
        .bind(&req.reason)
        .bind(&id)
        .execute(&s.db)
        .await?;
    let detail = if req.reason.is_empty() {
        format!("{id} (was {state})")
    } else {
        format!("{id} (was {state}): {}", req.reason)
    };
    s.audit(&c.name, "cancel", &detail).await;
    Ok(StatusCode::NO_CONTENT)
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
         WHERE u.round = ? AND u.kind != 'verify' AND u.state = ? AND u.manifest IS NOT NULL ORDER BY u.id",
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
    let row = sqlx::query(
        "SELECT u.round, u.state, u.unit, u.host, u.leased_at, u.done_at, h.token AS source_token \
         FROM units u JOIN hosts h ON h.id = u.host WHERE u.id = ? AND u.kind != 'verify'",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(|| conflict(format!("{id} isn't waiting for a verdict")))?;
    let target_round: i64 = row.get("round");
    let state: String = row.get("state");
    if state != "complete" {
        return Err(conflict(format!("{id} isn't waiting for a verdict")));
    }
    // cancel any pending verify units for this target since the trainer already judged it
    sqlx::query(
        "UPDATE units SET state = 'cancelled', host = NULL, lease_until = NULL \
         WHERE kind = 'verify' AND source_unit = ? AND state IN ('queued', 'leased')",
    )
    .bind(&id)
    .execute(&s.db)
    .await?;

    let source_host: i64 = row.get("host");
    let source_token: i64 = row.get("source_token");
    let target_work: WorkUnit =
        serde_json::from_str(row.get::<String, _>("unit").as_str()).map_err(|e| bad(e.to_string()))?;

    if v.accept {
        sqlx::query("UPDATE units SET state = 'accepted', verdict = ?, quarantined = 0 WHERE id = ?")
            .bind(&v.reason)
            .bind(&id)
            .execute(&s.db)
            .await?;
        let src_leased: Option<i64> = row.get("leased_at");
        let src_done: Option<i64> = row.get("done_at");
        let elapsed = match (src_leased, src_done) {
            (Some(l), Some(d)) => (d - l).max(1) as u64,
            _ => LEASE_SECS,
        };
        trust::award_credit(
            &s.db,
            trust::CreditGrant {
                token: source_token,
                host: Some(source_host),
                unit_id: &id,
                kind: "base",
                cost: target_work.cost,
                elapsed_secs: elapsed,
                window_secs: LEASE_SECS,
            },
        )
        .await?;
        if let Some(promoted) = trust::record_acceptance(&s.db, source_token, source_host, true).await? {
            s.audit("coordinator", "promote", &format!("token {source_token} -> {}", promoted.name())).await;
        }
        s.audit(&c.name, "accepted", &format!("{id}: {}", v.reason)).await;
    } else {
        let _ =
            trust::handle_mismatch(&s.db, source_token, source_host, &id, None, VerifyMode::Replay, &v.reason).await?;
        s.audit(&c.name, "rejected", &format!("{id}: {}", v.reason)).await;
    }
    gates::on_round_updated(&s, target_round).await?;
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
