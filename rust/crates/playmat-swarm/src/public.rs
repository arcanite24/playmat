//! Public tracker, live SSE stream, teams, contributor profiles, and transparency log (S3, docs/swarm/Design.md).
//! Every GET route here needs no token so `/swarm` in the web client is publicly readable.

use crate::store::now;
use crate::trust;
use crate::{bad, caller, conflict, require_not_banned, ApiError, Res, Shared, Swarm};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use futures_util::stream::{self, Stream};
use playmat_swarm_proto::api::*;
use serde::Deserialize;
use sqlx::Row;
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::time::Duration;

fn window_cutoff(window: Option<&str>) -> Option<i64> {
    let t = now() as i64;
    match window.unwrap_or("all") {
        "24h" => Some(t - 86_400),
        "7d" => Some(t - 7 * 86_400),
        "30d" => Some(t - 30 * 86_400),
        _ => None,
    }
}

pub(crate) async fn public_round(s: &Swarm, id: i64) -> Res<PublicRound> {
    let r = sqlx::query("SELECT project, run, round, priority, trained, created FROM rounds WHERE id = ?")
        .bind(id)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no round {id}")))?;
    let mut pr = PublicRound {
        id,
        project: r.get("project"),
        run: r.get("run"),
        round: r.get::<i64, _>("round") as u32,
        priority: r.get::<i64, _>("priority") as i32,
        trained: r.get::<i64, _>("trained") != 0,
        created: r.get("created"),
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
            pr.games += row.get::<i64, _>("g") as u64;
        }
        pr.units.insert(state, row.get::<i64, _>("n") as u32);
    }
    pr.verify_pending = sqlx::query(
        "SELECT COUNT(*) AS n FROM units WHERE round = ? AND kind = 'verify' AND state IN ('queued', 'leased')",
    )
    .bind(id)
    .fetch_one(&s.db)
    .await?
    .get::<i64, _>("n") as u32;
    Ok(pr)
}

pub(crate) async fn status_snapshot(s: &Swarm) -> Res<Status> {
    let since = now() as i64 - 600;
    let row = sqlx::query(
        "SELECT COUNT(*) AS n, COALESCE(SUM(threads), 0) AS t, \
         SUM(CASE WHEN gpu IS NOT NULL AND gpu != '' THEN 1 ELSE 0 END) AS g \
         FROM hosts WHERE seen >= ?",
    )
    .bind(since)
    .fetch_one(&s.db)
    .await?;
    let mut st = Status {
        hosts: row.get::<i64, _>("n") as u32,
        threads: row.get::<i64, _>("t") as u32,
        gpu_hosts: row.get::<Option<i64>, _>("g").unwrap_or(0) as u32,
        ..Default::default()
    };
    for r in sqlx::query("SELECT os, COUNT(*) AS n FROM hosts WHERE seen >= ? GROUP BY os")
        .bind(since)
        .fetch_all(&s.db)
        .await?
    {
        st.platforms.insert(r.get("os"), r.get::<i64, _>("n") as u32);
    }
    for r in sqlx::query("SELECT state, COUNT(*) AS n FROM units WHERE kind != 'verify' GROUP BY state")
        .fetch_all(&s.db)
        .await?
    {
        st.units.insert(r.get("state"), r.get::<i64, _>("n") as u32);
    }
    st.games_accepted =
        sqlx::query("SELECT COALESCE(SUM(games), 0) AS g FROM units WHERE kind != 'verify' AND state = 'accepted'")
            .fetch_one(&s.db)
            .await?
            .get::<i64, _>("g") as u64;
    st.games_last_hour = sqlx::query(
        "SELECT COALESCE(SUM(games), 0) AS g FROM units \
         WHERE kind != 'verify' AND state IN ('complete', 'accepted') AND done_at >= ?",
    )
    .bind(now() as i64 - 3600)
    .fetch_one(&s.db)
    .await?
    .get::<i64, _>("g") as u64;
    st.total_decisions = sqlx::query(
        "SELECT COALESCE(SUM(s.total_recorded), 0) AS d FROM unit_stats s \
         JOIN units u ON u.id = s.unit WHERE u.state = 'accepted'",
    )
    .fetch_one(&s.db)
    .await?
    .get::<i64, _>("d") as u64;
    st.open_inbox =
        sqlx::query("SELECT COUNT(*) AS n FROM inbox WHERE status = 'open'").fetch_one(&s.db).await?.get::<i64, _>("n")
            as u32;

    // highest-priority round with queued/leased/complete units, falling back to latest round
    let active_id: Option<i64> = sqlx::query_scalar(
        "SELECT r.id FROM rounds r \
         WHERE EXISTS (SELECT 1 FROM units u WHERE u.round = r.id AND u.kind != 'verify' \
                       AND u.state IN ('queued', 'leased', 'complete')) \
         ORDER BY r.priority DESC, r.id DESC LIMIT 1",
    )
    .fetch_optional(&s.db)
    .await?
    .or(sqlx::query_scalar("SELECT id FROM rounds ORDER BY id DESC LIMIT 1").fetch_optional(&s.db).await?);
    if let Some(rid) = active_id {
        st.active_round = Some(public_round(s, rid).await?);
    }
    Ok(st)
}

pub(crate) async fn status(State(s): State<Shared>) -> Res<Json<Status>> {
    status_snapshot(&s).await.map(Json)
}

/// Live SSE stream of public `Status` snapshots (one immediately, then every 5 seconds).
pub(crate) async fn events(State(s): State<Shared>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::unfold((s, true), |(s, first)| async move {
        if !first {
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
        let ev = match status_snapshot(&s).await {
            Ok(st) => Event::default().event("status").data(serde_json::to_string(&st).unwrap_or_default()),
            Err(_) => Event::default().comment("error"),
        };
        Some((Ok(ev), (s, false)))
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("ping"))
}

pub(crate) async fn projects(State(s): State<Shared>) -> Res<Json<Vec<PublicRound>>> {
    let rows = sqlx::query("SELECT id FROM rounds ORDER BY priority DESC, id DESC LIMIT 100").fetch_all(&s.db).await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(public_round(&s, r.get("id")).await?);
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
pub(crate) struct LimitQuery {
    limit: Option<i64>,
}

pub(crate) async fn feed(State(s): State<Shared>, Query(q): Query<LimitQuery>) -> Res<Json<Vec<FeedItem>>> {
    let limit = q.limit.unwrap_or(30).clamp(1, 100);
    let rows = sqlx::query(
        "SELECT u.id, r.project, r.run, r.round, u.kind, t.name AS account, h.name AS host, \
         u.games, u.credit, COALESCE(u.done_at, 0) AS done_at \
         FROM units u \
         JOIN rounds r ON r.id = u.round \
         JOIN hosts h ON h.id = u.host \
         JOIN tokens t ON t.id = h.token \
         WHERE u.state = 'accepted' \
         ORDER BY u.done_at DESC, u.id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(&s.db)
    .await?;
    let out = rows
        .into_iter()
        .map(|r| FeedItem {
            unit: r.get("id"),
            project: r.get("project"),
            run: r.get("run"),
            round: r.get::<i64, _>("round") as u32,
            kind: r.get("kind"),
            account: r.get("account"),
            host: r.get("host"),
            games: r.get::<i64, _>("games") as u64,
            credit: r.get("credit"),
            done_at: r.get("done_at"),
        })
        .collect();
    Ok(Json(out))
}

#[derive(Deserialize)]
pub(crate) struct WindowQuery {
    window: Option<String>,
}

pub(crate) async fn leaderboard(
    State(s): State<Shared>,
    Query(q): Query<WindowQuery>,
) -> Res<Json<Vec<LeaderboardEntry>>> {
    trust::mature_credits(&s.db).await?;
    let since = window_cutoff(q.window.as_deref()).unwrap_or(0);
    let rows = sqlx::query(
        "SELECT t.id, t.name, t.trust, tm.name AS team_name, \
         COALESCE((SELECT COUNT(*) FROM units u JOIN hosts h ON h.id = u.host \
                   WHERE h.token = t.id AND u.kind != 'verify' AND u.state = 'accepted' \
                   AND COALESCE(u.done_at, 0) >= ?), 0) AS accepted_units, \
         COALESCE((SELECT SUM(u.games) FROM units u JOIN hosts h ON h.id = u.host \
                   WHERE h.token = t.id AND u.kind != 'verify' AND u.state = 'accepted' \
                   AND COALESCE(u.done_at, 0) >= ?), 0) AS games, \
         COALESCE((SELECT COUNT(*) FROM units u JOIN hosts h ON h.id = u.host \
                   WHERE h.token = t.id AND u.kind = 'verify' AND u.state = 'accepted' \
                   AND COALESCE(u.done_at, 0) >= ?), 0) AS verified_units, \
         COALESCE((SELECT SUM(credit) FROM credits WHERE token = t.id AND at >= ? \
                   AND status IN ('pending', 'matured')), 0.0) AS credit, \
         COALESCE((SELECT SUM(credit) FROM credits WHERE token = t.id AND at >= ? \
                   AND status = 'pending'), 0.0) AS credit_pending, \
         COALESCE((SELECT SUM(credit) FROM credits WHERE token = t.id AND at >= ? \
                   AND status = 'matured'), 0.0) AS credit_matured, \
         COALESCE((SELECT SUM(credit) FROM credits WHERE token = t.id AND at >= ? \
                   AND status = 'frozen'), 0.0) AS credit_frozen \
         FROM tokens t LEFT JOIN teams tm ON tm.id = t.team_id \
         WHERE t.role = 'worker' \
         ORDER BY credit DESC, games DESC, accepted_units DESC, t.id ASC",
    )
    .bind(since)
    .bind(since)
    .bind(since)
    .bind(since)
    .bind(since)
    .bind(since)
    .bind(since)
    .fetch_all(&s.db)
    .await?;

    let out = rows
        .into_iter()
        .map(|r| LeaderboardEntry {
            account: r.get("name"),
            trust: TrustLevel::parse(r.get::<String, _>("trust").as_str()).unwrap_or(TrustLevel::Probation),
            team: r.get("team_name"),
            accepted_units: r.get::<i64, _>("accepted_units") as u64,
            games: r.get::<i64, _>("games") as u64,
            verified_units: r.get::<i64, _>("verified_units") as u64,
            credit: r.get("credit"),
            credit_pending: r.get("credit_pending"),
            credit_matured: r.get("credit_matured"),
            credit_frozen: r.get("credit_frozen"),
        })
        .collect();
    Ok(Json(out))
}

// ---- teams

pub(crate) async fn list_teams(State(s): State<Shared>, Query(q): Query<WindowQuery>) -> Res<Json<Vec<TeamInfo>>> {
    trust::mature_credits(&s.db).await?;
    let since = window_cutoff(q.window.as_deref()).unwrap_or(0);
    let rows = sqlx::query(
        "SELECT tm.id, tm.name, tm.description, tm.created, \
         (SELECT COUNT(*) FROM tokens t WHERE t.team_id = tm.id) AS members, \
         COALESCE((SELECT COUNT(*) FROM units u JOIN hosts h ON h.id = u.host JOIN tokens t ON t.id = h.token \
                   WHERE t.team_id = tm.id AND u.kind != 'verify' AND u.state = 'accepted' \
                   AND COALESCE(u.done_at, 0) >= MAX(?, COALESCE(t.team_joined_at, 0))), 0) AS accepted_units, \
         COALESCE((SELECT SUM(u.games) FROM units u JOIN hosts h ON h.id = u.host JOIN tokens t ON t.id = h.token \
                   WHERE t.team_id = tm.id AND u.kind != 'verify' AND u.state = 'accepted' \
                   AND COALESCE(u.done_at, 0) >= MAX(?, COALESCE(t.team_joined_at, 0))), 0) AS games, \
         COALESCE((SELECT SUM(c.credit) FROM credits c JOIN tokens t ON t.id = c.token \
                   WHERE t.team_id = tm.id AND c.status IN ('pending', 'matured') \
                   AND c.at >= MAX(?, COALESCE(t.team_joined_at, 0))), 0.0) AS credit \
         FROM teams tm ORDER BY credit DESC, games DESC, tm.id ASC",
    )
    .bind(since)
    .bind(since)
    .bind(since)
    .fetch_all(&s.db)
    .await?;

    let out = rows
        .into_iter()
        .map(|r| TeamInfo {
            id: r.get("id"),
            name: r.get("name"),
            description: r.get("description"),
            created: r.get("created"),
            members: r.get::<i64, _>("members") as u32,
            accepted_units: r.get::<i64, _>("accepted_units") as u64,
            games: r.get::<i64, _>("games") as u64,
            credit: r.get("credit"),
        })
        .collect();
    Ok(Json(out))
}

pub(crate) async fn create_team(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(req): Json<NewTeam>,
) -> Res<Json<TeamInfo>> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    require_not_banned(&c)?;
    let name = req.name.trim();
    if name.len() < 2 || name.len() > 48 {
        return Err(bad("team name must be 2 to 48 characters"));
    }
    let desc = req.description.trim();
    if desc.len() > 280 {
        return Err(bad("team description must be at most 280 characters"));
    }
    let t = now() as i64;
    let id: i64 =
        sqlx::query("INSERT INTO teams (name, description, created, owner_token) VALUES (?, ?, ?, ?) RETURNING id")
            .bind(name)
            .bind(desc)
            .bind(t)
            .bind(c.token)
            .fetch_one(&s.db)
            .await
            .map_err(|_| conflict(format!("team {name:?} already exists")))?
            .get("id");
    sqlx::query("UPDATE tokens SET team_id = ?, team_joined_at = ? WHERE id = ?")
        .bind(id)
        .bind(t)
        .bind(c.token)
        .execute(&s.db)
        .await?;
    s.audit(&c.name, "team-create", &format!("team {id} ({name})")).await;
    Ok(Json(TeamInfo {
        id,
        name: name.into(),
        description: desc.into(),
        created: t,
        members: 1,
        accepted_units: 0,
        games: 0,
        credit: 0.0,
    }))
}

pub(crate) async fn join_team(State(s): State<Shared>, headers: HeaderMap, Path(id): Path<i64>) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    require_not_banned(&c)?;
    let exists: i64 =
        sqlx::query("SELECT COUNT(*) AS n FROM teams WHERE id = ?").bind(id).fetch_one(&s.db).await?.get("n");
    if exists == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, format!("no team {id}")));
    }
    sqlx::query("UPDATE tokens SET team_id = ?, team_joined_at = ? WHERE id = ?")
        .bind(id)
        .bind(now() as i64)
        .bind(c.token)
        .execute(&s.db)
        .await?;
    s.audit(&c.name, "team-join", &format!("team {id}")).await;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn leave_team(State(s): State<Shared>, headers: HeaderMap) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    sqlx::query("UPDATE tokens SET team_id = NULL, team_joined_at = NULL WHERE id = ?")
        .bind(c.token)
        .execute(&s.db)
        .await?;
    s.audit(&c.name, "team-leave", "left team").await;
    Ok(StatusCode::NO_CONTENT)
}

// ---- contributor profile & badges

fn compute_badges(trust: TrustLevel, accepted_units: u64, games: u64, verified_units: u64) -> Vec<String> {
    let mut b = Vec::new();
    if accepted_units >= 1 {
        b.push("First Spark".into());
    }
    if accepted_units >= 20 {
        b.push("Probation Cleared".into());
    }
    if games >= 1_000 {
        b.push("1K Games".into());
    }
    if games >= 10_000 {
        b.push("10K Games".into());
    }
    if verified_units >= 10 {
        b.push("Verifier".into());
    }
    match trust {
        TrustLevel::Reputable => b.push("Reputable".into()),
        TrustLevel::Trusted => b.push("Trusted".into()),
        _ => {}
    }
    b
}

pub(crate) async fn contributor_profile(
    State(s): State<Shared>,
    Path(name): Path<String>,
) -> Res<Json<ContributorProfile>> {
    trust::mature_credits(&s.db).await?;
    let t = sqlx::query(
        "SELECT t.id, t.name, t.trust, t.created, tm.name AS team_name, \
         COALESCE((SELECT COUNT(*) FROM units u JOIN hosts h ON h.id = u.host \
                   WHERE h.token = t.id AND u.kind != 'verify' AND u.state = 'accepted'), 0) AS accepted_units, \
         COALESCE((SELECT SUM(u.games) FROM units u JOIN hosts h ON h.id = u.host \
                   WHERE h.token = t.id AND u.kind != 'verify' AND u.state = 'accepted'), 0) AS games, \
         COALESCE((SELECT COUNT(*) FROM units u JOIN hosts h ON h.id = u.host \
                   WHERE h.token = t.id AND u.kind = 'verify' AND u.state = 'accepted'), 0) AS verified_units, \
         COALESCE((SELECT SUM(credit) FROM credits WHERE token = t.id AND status IN ('pending', 'matured')), 0.0) AS credit, \
         COALESCE((SELECT SUM(credit) FROM credits WHERE token = t.id AND status = 'pending'), 0.0) AS credit_pending, \
         COALESCE((SELECT SUM(credit) FROM credits WHERE token = t.id AND status = 'matured'), 0.0) AS credit_matured, \
         COALESCE((SELECT SUM(credit) FROM credits WHERE token = t.id AND status = 'frozen'), 0.0) AS credit_frozen \
         FROM tokens t LEFT JOIN teams tm ON tm.id = t.team_id \
         WHERE t.name = ? AND t.role = 'worker' LIMIT 1",
    )
    .bind(&name)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no contributor {name}")))?;

    let tid: i64 = t.get("id");
    let trust = TrustLevel::parse(t.get::<String, _>("trust").as_str()).unwrap_or(TrustLevel::Probation);
    let accepted_units = t.get::<i64, _>("accepted_units") as u64;
    let games = t.get::<i64, _>("games") as u64;
    let verified_units = t.get::<i64, _>("verified_units") as u64;

    let hosts = sqlx::query(
        "SELECT name, os, class, threads, seen, accepted_units FROM hosts WHERE token = ? ORDER BY seen DESC",
    )
    .bind(tid)
    .fetch_all(&s.db)
    .await?
    .into_iter()
    .map(|h| ContributorHost {
        name: h.get("name"),
        os: h.get("os"),
        class: h.get("class"),
        threads: h.get::<i64, _>("threads") as u32,
        seen: h.get("seen"),
        accepted_units: h.get::<i64, _>("accepted_units") as u64,
    })
    .collect();

    let recent = sqlx::query(
        "SELECT u.id, r.project, r.run, r.round, u.kind, h.name AS host, u.games, u.credit, \
         COALESCE(u.done_at, 0) AS done_at \
         FROM units u JOIN rounds r ON r.id = u.round JOIN hosts h ON h.id = u.host \
         WHERE h.token = ? AND u.state = 'accepted' \
         ORDER BY u.done_at DESC, u.id DESC LIMIT 20",
    )
    .bind(tid)
    .fetch_all(&s.db)
    .await?
    .into_iter()
    .map(|r| FeedItem {
        unit: r.get("id"),
        project: r.get("project"),
        run: r.get("run"),
        round: r.get::<i64, _>("round") as u32,
        kind: r.get("kind"),
        account: name.clone(),
        host: r.get("host"),
        games: r.get::<i64, _>("games") as u64,
        credit: r.get("credit"),
        done_at: r.get("done_at"),
    })
    .collect();

    Ok(Json(ContributorProfile {
        account: t.get("name"),
        trust,
        team: t.get("team_name"),
        created: t.get("created"),
        accepted_units,
        games,
        verified_units,
        credit: t.get("credit"),
        credit_pending: t.get("credit_pending"),
        credit_matured: t.get("credit_matured"),
        credit_frozen: t.get("credit_frozen"),
        badges: compute_badges(trust, accepted_units, games, verified_units),
        hosts,
        recent,
    }))
}

// ---- transparency log (public run decisions & quarantine counts, no private moderation details)

pub(crate) async fn transparency(State(s): State<Shared>) -> Res<Json<TransparencyLog>> {
    let accepted_units =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM units WHERE kind != 'verify' AND state = 'accepted'")
            .fetch_one(&s.db)
            .await? as u64;
    let verified_units =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM units WHERE kind = 'verify' AND state = 'accepted'")
            .fetch_one(&s.db)
            .await? as u64;
    let rejected_units =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM units WHERE kind != 'verify' AND state = 'rejected'")
            .fetch_one(&s.db)
            .await? as u64;
    let quarantined_units =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM units WHERE kind != 'verify' AND state = 'quarantined'")
            .fetch_one(&s.db)
            .await? as u64;

    let rows = sqlx::query(
        "SELECT id, at, actor, action, detail FROM audit \
         WHERE action IN ('round', 'priority', 'cancel', 'trained', 'promote', 'verify-accept', 'verify-reject', \
                          'gate-pass', 'gate-fail', 'gate-hold', 'rollback', 'bench') \
         ORDER BY id DESC LIMIT 100",
    )
    .fetch_all(&s.db)
    .await?;
    let events = rows
        .into_iter()
        .map(|r| AuditEntry {
            id: r.get("id"),
            at: r.get("at"),
            actor: r.get("actor"),
            action: r.get("action"),
            detail: r.get("detail"),
        })
        .collect();
    Ok(Json(TransparencyLog { accepted_units, verified_units, rejected_units, quarantined_units, events }))
}

// ---- self-service volunteer worker signup (`POST /api/v1/register`)

const REGISTER_WINDOW_SECS: i64 = 3600;
const REGISTER_MAX_PER_HOUR: usize = 5;
const RESERVED_NAMES: &[&str] =
    &["admin", "administrator", "coordinator", "owner", "root", "staff", "system", "trainer", "verifier"];

fn client_ip(headers: &HeaderMap) -> String {
    if let Some(v) = headers.get("cf-connecting-ip").and_then(|h| h.to_str().ok()) {
        return v.trim().to_string();
    }
    if let Some(v) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
        if let Some(first) = v.split(',').next() {
            return first.trim().to_string();
        }
    }
    "local".to_string()
}

pub(crate) async fn register(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> Res<Json<TokenReply>> {
    if std::env::var("MANA_DISABLE_REGISTRATION").ok().as_deref() == Some("1") {
        return Err(ApiError(StatusCode::FORBIDDEN, "public worker registration is currently disabled".into()));
    }
    let name = req.name.trim();
    if !(3..=32).contains(&name.len())
        || name.starts_with('.')
        || name.starts_with('-')
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(bad("account name must be 3-32 characters of [A-Za-z0-9._-] and start with a letter or digit"));
    }
    let lower = name.to_ascii_lowercase();
    if RESERVED_NAMES.contains(&lower.as_str()) {
        return Err(bad("that account name is reserved"));
    }

    let ip = client_ip(&headers);
    let t = now() as i64;
    {
        let mut limits = s.reg_limits.lock().await;
        let entries = limits.entry(ip.clone()).or_default();
        entries.retain(|&ts| t - ts < REGISTER_WINDOW_SECS);
        if entries.len() >= REGISTER_MAX_PER_HOUR {
            return Err(ApiError(
                StatusCode::TOO_MANY_REQUESTS,
                "too many registrations from this address; try again later".into(),
            ));
        }
        entries.push(t);
    }

    let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tokens WHERE LOWER(name) = LOWER(?)")
        .bind(name)
        .fetch_one(&s.db)
        .await?;
    if existing > 0 {
        return Err(conflict(format!("account name '{name}' is already taken")));
    }

    let reply = s.new_token_with_trust(Role::Worker, name, Some(TrustLevel::Probation)).await?;
    s.audit(name, "register", &format!("self-service volunteer token {} registered", reply.id)).await;
    Ok(Json(reply))
}
