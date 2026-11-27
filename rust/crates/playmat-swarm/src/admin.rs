//! Admin and public trust/credit/inbox endpoints for `playmat-swarm` (S2, docs/swarm/Design.md).

use crate::store::now;
use crate::trust;
use crate::{bad, caller, conflict, spawn_verify_unit, ApiError, Res, Shared};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use playmat_swarm_proto::api::*;
use playmat_swarm_proto::VerifyMode;
use serde::Deserialize;
use sqlx::Row;

// ---- admin workers & tokens

pub(crate) async fn list_workers(State(s): State<Shared>, headers: HeaderMap) -> Res<Json<Vec<WorkerInfo>>> {
    caller(&s, &headers, &[Role::Admin]).await?;
    trust::mature_credits(&s.db).await?;
    let tokens = sqlx::query(
        "SELECT id, name, role, trust, created, revoked, accepted_units, mismatches, extra_audit_until, \
         COALESCE((SELECT SUM(credit) FROM credits WHERE token = tokens.id AND status IN ('pending', 'matured')), 0.0) AS credit_total, \
         COALESCE((SELECT SUM(credit) FROM credits WHERE token = tokens.id AND status = 'pending'), 0.0) AS credit_pending, \
         COALESCE((SELECT SUM(credit) FROM credits WHERE token = tokens.id AND status = 'matured'), 0.0) AS credit_matured, \
         COALESCE((SELECT SUM(credit) FROM credits WHERE token = tokens.id AND status = 'frozen'), 0.0) AS credit_frozen \
         FROM tokens ORDER BY id ASC",
    )
    .fetch_all(&s.db)
    .await?;

    let mut out = Vec::with_capacity(tokens.len());
    for t in tokens {
        let tid: i64 = t.get("id");
        let host_rows = sqlx::query(
            "SELECT id, name, build, class, strict, threads, os, gpu, seen, accepted_units, mismatches, probation_passed \
             FROM hosts WHERE token = ? ORDER BY id ASC",
        )
        .bind(tid)
        .fetch_all(&s.db)
        .await?;
        let hosts = host_rows
            .into_iter()
            .map(|h| HostInfo {
                id: h.get("id"),
                name: h.get("name"),
                build: h.get("build"),
                class: h.get("class"),
                strict: h.get::<i64, _>("strict") != 0,
                threads: h.get::<i64, _>("threads") as u32,
                os: h.get("os"),
                gpu: h.get("gpu"),
                seen: h.get("seen"),
                accepted_units: h.get::<i64, _>("accepted_units") as u64,
                mismatches: h.get::<i64, _>("mismatches") as u64,
                probation_passed: h.get::<i64, _>("probation_passed") != 0,
            })
            .collect();
        out.push(WorkerInfo {
            id: tid,
            name: t.get("name"),
            role: Role::parse(t.get::<String, _>("role").as_str()).unwrap_or(Role::Worker),
            trust: TrustLevel::parse(t.get::<String, _>("trust").as_str()).unwrap_or(TrustLevel::Probation),
            created: t.get("created"),
            revoked: t.get::<i64, _>("revoked") != 0,
            accepted_units: t.get::<i64, _>("accepted_units") as u64,
            mismatches: t.get::<i64, _>("mismatches") as u64,
            extra_audit_until: t.get("extra_audit_until"),
            credit_total: t.get("credit_total"),
            credit_pending: t.get("credit_pending"),
            credit_matured: t.get("credit_matured"),
            credit_frozen: t.get("credit_frozen"),
            hosts,
        });
    }
    Ok(Json(out))
}

pub(crate) async fn set_token_trust(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<SetTrust>,
) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Admin]).await?;
    let n = sqlx::query("UPDATE tokens SET trust = ? WHERE id = ?")
        .bind(req.trust.name())
        .bind(id)
        .execute(&s.db)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, format!("no token {id}")));
    }
    if req.trust == TrustLevel::Trusted {
        sqlx::query("UPDATE hosts SET probation_passed = 1 WHERE token = ?").bind(id).execute(&s.db).await?;
    }
    s.audit(&c.name, "trust", &format!("token {id} -> {}: {}", req.trust.name(), req.reason)).await;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn revoke_token(State(s): State<Shared>, headers: HeaderMap, Path(id): Path<i64>) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Admin]).await?;
    let n = sqlx::query("UPDATE tokens SET revoked = 1 WHERE id = ?").bind(id).execute(&s.db).await?.rows_affected();
    if n == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, format!("no token {id}")));
    }
    s.audit(&c.name, "revoke", &format!("token {id}")).await;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn quarantine_token(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<QuarantineRequest>,
) -> Res<Json<QuarantineReply>> {
    let c = caller(&s, &headers, &[Role::Admin]).await?;
    let exists: i64 =
        sqlx::query("SELECT COUNT(*) AS n FROM tokens WHERE id = ?").bind(id).fetch_one(&s.db).await?.get("n");
    if exists == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, format!("no token {id}")));
    }
    let units_affected = if let Some(round_id) = req.round_id {
        sqlx::query(
            "UPDATE units SET state = 'quarantined', quarantined = 1 \
             WHERE round = ? AND kind != 'verify' AND state IN ('complete', 'accepted') \
             AND host IN (SELECT id FROM hosts WHERE token = ?)",
        )
        .bind(round_id)
        .bind(id)
        .execute(&s.db)
        .await?
        .rows_affected() as u32
    } else {
        sqlx::query(
            "UPDATE units SET state = 'quarantined', quarantined = 1 \
             WHERE kind != 'verify' AND state IN ('complete', 'accepted') \
             AND host IN (SELECT id FROM hosts WHERE token = ?) \
             AND round IN (SELECT id FROM rounds WHERE trained = 0)",
        )
        .bind(id)
        .execute(&s.db)
        .await?
        .rows_affected() as u32
    };
    let t = now() as i64;
    let credits_affected = sqlx::query(
        "UPDATE credits SET status = 'frozen' WHERE token = ? AND at >= ? AND status IN ('pending', 'matured')",
    )
    .bind(id)
    .bind(t - trust::CREDIT_FREEZE_WINDOW_SECS)
    .execute(&s.db)
    .await?
    .rows_affected() as u32;
    sqlx::query(
        "UPDATE units SET credit_status = 'frozen' \
         WHERE host IN (SELECT id FROM hosts WHERE token = ?) AND done_at >= ? AND credit_status IN ('pending', 'matured')",
    )
    .bind(id)
    .bind(t - trust::CREDIT_FREEZE_WINDOW_SECS)
    .execute(&s.db)
    .await?;
    s.audit(
        &c.name,
        "quarantine",
        &format!(
            "token {id} (round={:?}): {units_affected} units, {credits_affected} credits: {}",
            req.round_id, req.reason
        ),
    )
    .await;
    Ok(Json(QuarantineReply { units_affected, credits_affected }))
}

pub(crate) async fn restore_token(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<QuarantineRequest>,
) -> Res<Json<QuarantineReply>> {
    let c = caller(&s, &headers, &[Role::Admin]).await?;
    let units_affected = restore_account_units(&s, id, req.round_id).await?;
    let credits_affected = unfreeze_account_credits(&s, id).await?;
    s.audit(
        &c.name,
        "restore",
        &format!(
            "token {id} (round={:?}): {units_affected} units, {credits_affected} credits: {}",
            req.round_id, req.reason
        ),
    )
    .await;
    Ok(Json(QuarantineReply { units_affected, credits_affected }))
}

async fn restore_account_units(s: &Shared, token_id: i64, round_id: Option<i64>) -> Result<u32, sqlx::Error> {
    let n = if let Some(rid) = round_id {
        sqlx::query(
            "UPDATE units SET state = CASE WHEN credit > 0 THEN 'accepted' ELSE 'complete' END, quarantined = 0 \
             WHERE round = ? AND state = 'quarantined' AND host IN (SELECT id FROM hosts WHERE token = ?)",
        )
        .bind(rid)
        .bind(token_id)
        .execute(&s.db)
        .await?
        .rows_affected()
    } else {
        sqlx::query(
            "UPDATE units SET state = CASE WHEN credit > 0 THEN 'accepted' ELSE 'complete' END, quarantined = 0 \
             WHERE state = 'quarantined' AND host IN (SELECT id FROM hosts WHERE token = ?)",
        )
        .bind(token_id)
        .execute(&s.db)
        .await?
        .rows_affected()
    };
    Ok(n as u32)
}

async fn unfreeze_account_credits(s: &Shared, token_id: i64) -> Result<u32, sqlx::Error> {
    let t = now() as i64;
    let n = sqlx::query(
        "UPDATE credits SET status = CASE WHEN mature_at <= ? THEN 'matured' ELSE 'pending' END \
         WHERE token = ? AND status = 'frozen'",
    )
    .bind(t)
    .bind(token_id)
    .execute(&s.db)
    .await?
    .rows_affected();
    sqlx::query(
        "UPDATE units SET credit_status = CASE WHEN credit_mature_at <= ? THEN 'matured' ELSE 'pending' END \
         WHERE host IN (SELECT id FROM hosts WHERE token = ?) AND credit_status = 'frozen'",
    )
    .bind(t)
    .bind(token_id)
    .execute(&s.db)
    .await?;
    Ok(n as u32)
}

pub(crate) async fn adjust_credit(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<CreditAdjustment>,
) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Admin]).await?;
    if !req.credit.is_finite() || req.reason.trim().is_empty() {
        return Err(bad("a finite credit adjustment and a written reason are required"));
    }
    let exists: i64 =
        sqlx::query("SELECT COUNT(*) AS n FROM tokens WHERE id = ?").bind(id).fetch_one(&s.db).await?.get("n");
    if exists == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, format!("no token {id}")));
    }
    let t = now() as i64;
    sqlx::query(
        "INSERT INTO credits (at, token, host, unit, kind, base_credit, bonus_mult, credit, status, mature_at, reason) \
         VALUES (?, ?, NULL, NULL, 'adjustment', ?, 1.0, ?, 'matured', ?, ?)",
    )
    .bind(t)
    .bind(id)
    .bind(req.credit)
    .bind(req.credit)
    .bind(t)
    .bind(&req.reason)
    .execute(&s.db)
    .await?;
    s.audit(&c.name, "credit", &format!("token {id} {:+.2}: {}", req.credit, req.reason)).await;
    Ok(StatusCode::NO_CONTENT)
}

// ---- rounds / units admin actions

pub(crate) async fn mark_round_trained(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    let n = sqlx::query("UPDATE rounds SET trained = 1 WHERE id = ?").bind(id).execute(&s.db).await?.rows_affected();
    if n == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, format!("no round {id}")));
    }
    s.audit(&c.name, "trained", &format!("round {id}")).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Deletes R2/local `intake/{round}/{unit}/*` objects after a round has been trained and archived to Hugging Face.
pub(crate) async fn purge_round_intake(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<PurgeIntakeRequest>,
) -> Res<Json<PurgeIntakeReply>> {
    let c = caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    let round = sqlx::query("SELECT trained FROM rounds WHERE id = ?")
        .bind(id)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no round {id}")))?;
    let trained: i64 = round.get("trained");
    if trained == 0 && !req.force {
        return Err(conflict(format!(
            "round {id} is not marked trained yet; mark it trained first or pass force=true"
        )));
    }
    let states: &[&str] = if req.include_rejected { &["accepted", "rejected", "cancelled"] } else { &["accepted"] };
    let rows = sqlx::query(
        "SELECT id, state, manifest FROM units WHERE round = ? AND kind != 'verify' AND manifest IS NOT NULL",
    )
    .bind(id)
    .fetch_all(&s.db)
    .await?;

    let mut units_purged = 0u32;
    let mut files_deleted = 0u32;
    for row in rows {
        let uid: String = row.get("id");
        let state: String = row.get("state");
        if !states.contains(&state.as_str()) {
            continue;
        }
        let manifest_str: String = row.get("manifest");
        let Ok(manifest) = serde_json::from_str::<playmat_swarm_proto::ResultManifest>(&manifest_str) else {
            continue;
        };
        let mut unit_removed = false;
        for file in manifest.files.keys() {
            let key = format!("intake/{id}/{uid}/{file}");
            if s.store.delete(&key).await? {
                files_deleted += 1;
                unit_removed = true;
            }
        }
        if unit_removed {
            units_purged += 1;
        }
    }
    sqlx::query("UPDATE rounds SET archived = 1 WHERE id = ?").bind(id).execute(&s.db).await?;
    let detail = if req.archive_ref.trim().is_empty() {
        format!("round {id}: purged {files_deleted} files across {units_purged} units")
    } else {
        format!(
            "round {id}: purged {files_deleted} files across {units_purged} units (archive={})",
            req.archive_ref.trim()
        )
    };
    s.audit(&c.name, "purge-intake", &detail).await;
    Ok(Json(PurgeIntakeReply { round_id: id, units_purged, files_deleted }))
}

pub(crate) async fn order_unit_rerun(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<OrderRerun>,
) -> Res<Json<serde_json::Value>> {
    let c = caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    let vid = spawn_verify_unit(&s, &id, VerifyMode::Rerun, true).await?;
    s.audit(&c.name, "order-rerun", &format!("{id} -> {vid}: {}", req.reason)).await;
    Ok(Json(serde_json::json!({ "verify_unit": vid })))
}

pub(crate) async fn reissue_unit(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    let _one = s.lease.lock().await;
    let row = sqlx::query("SELECT state, kind FROM units WHERE id = ?")
        .bind(&id)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no unit {id}")))?;
    let state: String = row.get("state");
    let kind: String = row.get("kind");
    if kind == "verify" {
        return Err(bad("can't reissue a verify unit directly"));
    }
    if !matches!(state.as_str(), "failed" | "rejected" | "quarantined" | "cancelled") {
        return Err(conflict(format!("{id} is {state}, can't reissue")));
    }
    sqlx::query(
        "UPDATE units SET state = 'queued', host = NULL, lease_until = NULL, leased_at = NULL, \
         attempts = 0, manifest = NULL, games = 0, done_at = NULL, verdict = NULL, quarantined = 0 \
         WHERE id = ?",
    )
    .bind(&id)
    .execute(&s.db)
    .await?;
    sqlx::query("DELETE FROM failures WHERE unit = ?").bind(&id).execute(&s.db).await?;
    sqlx::query("DELETE FROM uploads WHERE unit = ?").bind(&id).execute(&s.db).await?;
    s.audit(&c.name, "reissue", &format!("{id} (was {state})")).await;
    Ok(StatusCode::NO_CONTENT)
}

// ---- admin verification inbox

#[derive(Deserialize)]
pub(crate) struct InboxQuery {
    status: Option<String>,
}

pub(crate) async fn list_inbox(
    State(s): State<Shared>,
    headers: HeaderMap,
    Query(q): Query<InboxQuery>,
) -> Res<Json<Vec<InboxItem>>> {
    caller(&s, &headers, &[Role::Admin]).await?;
    let rows = if let Some(st) = q.status {
        sqlx::query(
            "SELECT i.id, i.created, i.status, i.kind, i.token, t.name AS account, i.host, h.name AS host_name, \
             i.unit, i.verify_unit, i.detail, i.resolved_by, i.resolved_at, i.resolution \
             FROM inbox i LEFT JOIN tokens t ON t.id = i.token LEFT JOIN hosts h ON h.id = i.host \
             WHERE i.status = ? ORDER BY i.id DESC LIMIT 200",
        )
        .bind(st)
        .fetch_all(&s.db)
        .await?
    } else {
        sqlx::query(
            "SELECT i.id, i.created, i.status, i.kind, i.token, t.name AS account, i.host, h.name AS host_name, \
             i.unit, i.verify_unit, i.detail, i.resolved_by, i.resolved_at, i.resolution \
             FROM inbox i LEFT JOIN tokens t ON t.id = i.token LEFT JOIN hosts h ON h.id = i.host \
             ORDER BY i.id DESC LIMIT 200",
        )
        .fetch_all(&s.db)
        .await?
    };

    let out = rows
        .into_iter()
        .map(|r| InboxItem {
            id: r.get("id"),
            created: r.get("created"),
            status: r.get("status"),
            kind: r.get("kind"),
            token: r.get("token"),
            account: r.get("account"),
            host: r.get("host"),
            host_name: r.get("host_name"),
            unit: r.get("unit"),
            verify_unit: r.get("verify_unit"),
            detail: r.get("detail"),
            resolved_by: r.get("resolved_by"),
            resolved_at: r.get("resolved_at"),
            resolution: r.get("resolution"),
        })
        .collect();
    Ok(Json(out))
}

pub(crate) async fn resolve_inbox(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<ResolveInbox>,
) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Admin]).await?;
    let row = sqlx::query("SELECT status, token, unit FROM inbox WHERE id = ?")
        .bind(id)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no inbox item {id}")))?;
    let status: String = row.get("status");
    if status != "open" {
        return Err(conflict(format!("inbox item {id} is already {status}")));
    }
    let token: Option<i64> = row.get("token");
    let unit: Option<String> = row.get("unit");
    let next_status = match req.action.as_str() {
        "confirm-cheat" => {
            if let Some(tid) = token {
                sqlx::query("UPDATE tokens SET trust = 'banned', revoked = 1 WHERE id = ?")
                    .bind(tid)
                    .execute(&s.db)
                    .await?;
                sqlx::query("UPDATE credits SET status = 'revoked' WHERE token = ? AND status = 'frozen'")
                    .bind(tid)
                    .execute(&s.db)
                    .await?;
                sqlx::query(
                    "UPDATE units SET credit_status = 'revoked' \
                     WHERE host IN (SELECT id FROM hosts WHERE token = ?) AND credit_status = 'frozen'",
                )
                .bind(tid)
                .execute(&s.db)
                .await?;
            }
            "resolved"
        }
        "false-positive" => {
            if let Some(tid) = token {
                restore_account_units(&s, tid, None).await?;
                unfreeze_account_credits(&s, tid).await?;
                sqlx::query("UPDATE tokens SET extra_audit_until = 0 WHERE id = ?").bind(tid).execute(&s.db).await?;
            }
            "resolved"
        }
        "order-rerun" => {
            let uid = unit.ok_or_else(|| bad("inbox item has no unit to rerun"))?;
            let _ = spawn_verify_unit(&s, &uid, VerifyMode::Rerun, true).await?;
            "resolved"
        }
        "dismiss" => "dismissed",
        other => {
            return Err(bad(format!(
                "unknown action {other:?}: expected confirm-cheat, false-positive, order-rerun, or dismiss"
            )));
        }
    };

    let resolution = if req.reason.is_empty() { req.action.clone() } else { format!("{}: {}", req.action, req.reason) };
    sqlx::query("UPDATE inbox SET status = ?, resolved_by = ?, resolved_at = ?, resolution = ? WHERE id = ?")
        .bind(next_status)
        .bind(&c.name)
        .bind(now() as i64)
        .bind(&resolution)
        .bind(id)
        .execute(&s.db)
        .await?;
    s.audit(&c.name, "inbox", &format!("item {id} -> {next_status} ({resolution})")).await;
    Ok(StatusCode::NO_CONTENT)
}

// ---- admin audit log

#[derive(Deserialize)]
pub(crate) struct AuditQuery {
    action: Option<String>,
    actor: Option<String>,
    limit: Option<i64>,
}

pub(crate) async fn list_audit(
    State(s): State<Shared>,
    headers: HeaderMap,
    Query(q): Query<AuditQuery>,
) -> Res<Json<Vec<AuditEntry>>> {
    caller(&s, &headers, &[Role::Admin]).await?;
    let limit = q.limit.unwrap_or(100).clamp(1, 1000);
    let rows = sqlx::query(
        "SELECT id, at, actor, action, detail FROM audit \
         WHERE (? IS NULL OR action = ?) AND (? IS NULL OR actor = ?) \
         ORDER BY id DESC LIMIT ?",
    )
    .bind(&q.action)
    .bind(&q.action)
    .bind(&q.actor)
    .bind(&q.actor)
    .bind(limit)
    .fetch_all(&s.db)
    .await?;
    let out = rows
        .into_iter()
        .map(|r| AuditEntry {
            id: r.get("id"),
            at: r.get("at"),
            actor: r.get("actor"),
            action: r.get("action"),
            detail: r.get("detail"),
        })
        .collect();
    Ok(Json(out))
}
