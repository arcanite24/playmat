//! Mana@Home S4: SPRT swarm gates, Archbench rating units, automatic promotion, hold and rollback (docs/swarm/Design.md).

use crate::store::{self, now};
use crate::{bad, caller, conflict, spawn_verify_unit, trust, valid_sha, ApiError, Res, Shared, Swarm};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use playmat_swarm_proto::api::*;
use playmat_swarm_proto::{self as proto, ResultManifest, UnitKind, VerifyMode, WorkUnit};
use sqlx::Row;
use std::collections::BTreeMap;

/// Expected logistic score in `(0, 1)` for a given Elo difference.
pub fn logistic_score(elo: f64) -> f64 {
    1.0 / (1.0 + 10.0f64.powf(-elo / 400.0))
}

/// Converts a win-rate score in `[0, 1]` to a logistic Elo difference (clamped to avoid infinities).
pub fn score_to_elo(score: f64) -> f64 {
    let s = score.clamp(1e-4, 1.0 - 1e-4);
    -400.0 * (1.0 / s - 1.0).log10()
}

/// Returns `(elo_diff, ci95_half_width)` for `(wins, draws, losses)`.
pub fn elo_and_ci95(wins: u64, draws: u64, losses: u64) -> (f64, f64) {
    let n = (wins + draws + losses) as f64;
    if n < 1.0 {
        return (0.0, 0.0);
    }
    let p = (wins as f64 + 0.5 * draws as f64) / n;
    let var =
        ((wins as f64) * (1.0 - p).powi(2) + (draws as f64) * (0.5 - p).powi(2) + (losses as f64) * p.powi(2)) / n;
    let se = (var / n).sqrt().max(1e-6);
    let elo = score_to_elo(p);
    let lo = score_to_elo((p - 1.96 * se).clamp(1e-4, 1.0 - 1e-4));
    let hi = score_to_elo((p + 1.96 * se).clamp(1e-4, 1.0 - 1e-4));
    (elo, (hi - lo) * 0.5)
}

/// Wald SPRT `(lower, upper)` log-likelihood ratio bounds for `(alpha, beta)`.
pub fn sprt_bounds(alpha: f64, beta: f64) -> (f64, f64) {
    let a = alpha.clamp(1e-6, 0.49);
    let b = beta.clamp(1e-6, 0.49);
    ((b / (1.0 - a)).ln(), ((1.0 - b) / a).ln())
}

/// Generalized trinomial score LLR (Fishtest / cutechess style) for H0: `elo0` vs H1: `elo1`.
pub fn sprt_llr(wins: u64, draws: u64, losses: u64, elo0: f64, elo1: f64) -> f64 {
    let n = (wins + draws + losses) as f64;
    if n < 2.0 {
        return 0.0;
    }
    let p = (wins as f64 + 0.5 * draws as f64) / n;
    let s0 = logistic_score(elo0);
    let s1 = logistic_score(elo1);
    let var = (((wins as f64) * (1.0 - p).powi(2) + (draws as f64) * (0.5 - p).powi(2) + (losses as f64) * p.powi(2))
        / n)
        .max(1e-4);
    0.5 * n * (((p - s0).powi(2) + var) / ((p - s1).powi(2) + var)).ln()
}

/// Registers a network in `networks` if not already present (idempotent).
async fn ensure_network(s: &Swarm, sha: &str, label: &str, parent_sha: Option<&str>, status: &str) -> Res<()> {
    sqlx::query("INSERT OR IGNORE INTO networks (sha256, label, parent_sha, status, created) VALUES (?, ?, ?, ?, ?)")
        .bind(sha)
        .bind(label)
        .bind(parent_sha)
        .bind(status)
        .bind(now() as i64)
        .execute(&s.db)
        .await?;
    Ok(())
}

/// Validates and inserts a round's units (shared by `POST /gates` and `POST /bench`).
async fn insert_typed_round(
    s: &Swarm,
    project: &str,
    run: &str,
    round: u32,
    priority: i32,
    expected_kind: UnitKind,
    mut units: Vec<WorkUnit>,
) -> Res<i64> {
    if units.is_empty() {
        return Err(bad("a round with no units"));
    }
    for u in &units {
        proto::check_unit(u).map_err(|e| bad(format!("{}: {e}", u.id)))?;
        if u.kind != expected_kind {
            return Err(bad(format!("{}: expected {:?} unit, got {:?}", u.id, expected_kind, u.kind)));
        }
        if u.project != project || u.run != run || u.round != round {
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
    for u in &mut units {
        for n in &mut u.nets {
            n.url = format!("{}/api/v1/nets/{}", s.base.trim_end_matches('/'), n.sha256);
        }
    }
    let mut tx = s.db.begin().await?;
    let round_id: i64 =
        sqlx::query("INSERT INTO rounds (project, run, round, priority, created) VALUES (?, ?, ?, ?, ?) RETURNING id")
            .bind(project)
            .bind(run)
            .bind(round as i64)
            .bind(priority as i64)
            .bind(now() as i64)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| conflict(format!("round {round} of {project}/{run}: {e}")))?
            .get("id");
    for u in &units {
        sqlx::query("INSERT INTO units (id, round, unit, state, kind) VALUES (?, ?, ?, 'queued', ?)")
            .bind(&u.id)
            .bind(round_id)
            .bind(serde_json::to_string(u).unwrap())
            .bind(trust::unit_kind_name(u.kind))
            .execute(&mut *tx)
            .await
            .map_err(|e| conflict(format!("unit {}: {e}", u.id)))?;
    }
    tx.commit().await?;
    Ok(round_id)
}

async fn promote_network_tx(s: &Swarm, sha: &str, actor: &str, reason: &str) -> Res<()> {
    let t = now() as i64;
    sqlx::query("UPDATE networks SET status = 'superseded' WHERE status = 'promoted' AND sha256 != ?")
        .bind(sha)
        .execute(&s.db)
        .await?;
    sqlx::query("UPDATE networks SET status = 'promoted', promoted_at = ? WHERE sha256 = ?")
        .bind(t)
        .bind(sha)
        .execute(&s.db)
        .await?;
    s.audit(actor, "promote", &format!("network {sha}: {reason}")).await;
    Ok(())
}

/// Called after any unit in `round_id` is accepted, rejected, or verified (including cross-account re-runs).
/// Updates the associated gate (SPRT + concentration guard + auto-promotion) or Archbench rating round if any.
pub(crate) async fn on_round_updated(s: &Swarm, round_id: i64) -> Res<()> {
    if let Some(gid) = sqlx::query_scalar::<_, i64>("SELECT id FROM gates WHERE round_id = ?")
        .bind(round_id)
        .fetch_optional(&s.db)
        .await?
    {
        evaluate_gate(s, gid).await?;
    }
    if let Some(bid) = sqlx::query_scalar::<_, i64>("SELECT id FROM benches WHERE round_id = ?")
        .bind(round_id)
        .fetch_optional(&s.db)
        .await?
    {
        evaluate_bench(s, bid).await?;
    }
    Ok(())
}

/// Recomputes SPRT statistics from all `accepted` match units, enforces single-account concentration re-runs,
/// cancels leftover units on early stop, and auto-promotes when passed and not held.
pub(crate) async fn evaluate_gate(s: &Swarm, gate_id: i64) -> Res<()> {
    let row = sqlx::query(
        "SELECT id, round_id, candidate_sha, candidate_label, status, held, auto_promote, \
         elo0, elo1, alpha, beta, max_share FROM gates WHERE id = ?",
    )
    .bind(gate_id)
    .fetch_optional(&s.db)
    .await?;
    let Some(row) = row else { return Ok(()) };

    let round_id: i64 = row.get("round_id");
    let candidate_sha: String = row.get("candidate_sha");
    let candidate_label: String = row.get("candidate_label");
    let current_status: String = row.get("status");
    let held = row.get::<i64, _>("held") != 0;
    let auto_promote = row.get::<i64, _>("auto_promote") != 0;
    let elo0: f64 = row.get("elo0");
    let elo1: f64 = row.get("elo1");
    let alpha: f64 = row.get("alpha");
    let beta: f64 = row.get("beta");
    let max_share: f64 = row.get("max_share");

    // Load all accepted match units in this gate round, with their account and whether a cross-account rerun passed
    let urows = sqlx::query(
        "SELECT u.id, u.manifest, h.token AS account_token, \
         EXISTS(SELECT 1 FROM units v WHERE v.source_unit = u.id AND v.kind = 'verify' \
                AND v.verify_mode = 'rerun' AND v.state = 'accepted') AS cross_rerun \
         FROM units u JOIN hosts h ON h.id = u.host \
         WHERE u.round = ? AND u.kind = 'match' AND u.state = 'accepted' AND u.manifest IS NOT NULL \
         ORDER BY u.id",
    )
    .bind(round_id)
    .fetch_all(&s.db)
    .await?;

    let mut wins = 0u64;
    let mut draws = 0u64;
    let mut losses = 0u64;
    // Un-reverified games per account (for single-account concentration guard)
    let mut unverified_by_account: BTreeMap<i64, Vec<(String, u64)>> = BTreeMap::new();
    let mut total_games = 0u64;

    for ur in &urows {
        let uid: String = ur.get("id");
        let acc: i64 = ur.get("account_token");
        let cross_rerun = ur.get::<i64, _>("cross_rerun") != 0;
        let manifest: ResultManifest =
            serde_json::from_str(ur.get::<String, _>("manifest").as_str()).map_err(|e| bad(e.to_string()))?;
        let mut ug = 0u64;
        for g in &manifest.games {
            ug += 1;
            match g.winner {
                None => draws += 1,
                Some(w) if w == g.learner => wins += 1,
                _ => losses += 1,
            }
        }
        total_games += ug;
        if !cross_rerun {
            unverified_by_account.entry(acc).or_default().push((uid, ug));
        }
    }

    let (elo, ci95) = elo_and_ci95(wins, draws, losses);
    let llr = sprt_llr(wins, draws, losses, elo0, elo1);
    let (lower, upper) = sprt_bounds(alpha, beta);

    let (dominant_account, dominant_games) = unverified_by_account
        .iter()
        .map(|(&acc, list)| (acc, list.iter().map(|(_, g)| *g).sum::<u64>()))
        .max_by_key(|&(_, g)| g)
        .unwrap_or((0, 0));
    let top_share = if total_games > 0 { dominant_games as f64 / total_games as f64 } else { 0.0 };

    // Always persist latest live W/D/L, Elo, LLR, and share
    sqlx::query(
        "UPDATE gates SET wins = ?, draws = ?, losses = ?, elo = ?, elo_ci95 = ?, llr = ?, top_account_share = ? \
         WHERE id = ?",
    )
    .bind(wins as i64)
    .bind(draws as i64)
    .bind(losses as i64)
    .bind(elo)
    .bind(ci95)
    .bind(llr)
    .bind(top_share)
    .bind(gate_id)
    .execute(&s.db)
    .await?;

    // Don't re-decide already finalized gates
    if matches!(current_status.as_str(), "promoted" | "failed" | "rolled-back") {
        return Ok(());
    }

    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM units WHERE round = ? AND kind = 'match' AND state IN ('queued', 'leased', 'complete')",
    )
    .bind(round_id)
    .fetch_one(&s.db)
    .await?;

    if llr >= upper {
        // Cancel remaining queued match units (early stopping)
        cancel_queued_units(s, round_id, "SPRT passed (early stop)").await?;

        // Single-account concentration guard: if one account has > max_share un-rerun games, trigger cross-account re-runs
        if top_share > max_share {
            let mut need_games = (dominant_games as f64 - max_share * total_games as f64).ceil().max(1.0) as i64;
            let mut spawned = 0u32;
            if let Some(list) = unverified_by_account.get(&dominant_account) {
                for (uid, ug) in list {
                    if need_games <= 0 {
                        break;
                    }
                    // Skip if a rerun verify unit is already queued/leased for this unit
                    let already: i64 = sqlx::query_scalar(
                        "SELECT COUNT(*) FROM units WHERE source_unit = ? AND kind = 'verify' \
                         AND verify_mode = 'rerun' AND state IN ('queued', 'leased')",
                    )
                    .bind(uid)
                    .fetch_one(&s.db)
                    .await?;
                    if already == 0 {
                        let _ = spawn_verify_unit(s, uid, VerifyMode::Rerun, true).await?;
                        spawned += 1;
                    }
                    need_games -= *ug as i64;
                }
            }
            let reason = format!(
                "SPRT passed (LLR {llr:+.2} >= {upper:.2}), awaiting cross-account re-run (top share {:.0}% > {:.0}%)",
                top_share * 100.0,
                max_share * 100.0
            );
            sqlx::query("UPDATE gates SET status = 'concentration-hold', decision_reason = ? WHERE id = ?")
                .bind(&reason)
                .bind(gate_id)
                .execute(&s.db)
                .await?;
            if spawned > 0 {
                s.audit("coordinator", "gate-hold", &format!("gate {gate_id}: {reason}; queued {spawned} re-run(s)"))
                    .await;
            }
            return Ok(());
        }

        if held {
            let reason = format!("SPRT passed (LLR {llr:+.2}), held by admin");
            sqlx::query("UPDATE gates SET status = 'held', decision_reason = ? WHERE id = ?")
                .bind(&reason)
                .bind(gate_id)
                .execute(&s.db)
                .await?;
            return Ok(());
        }

        let t = now() as i64;
        let reason =
            format!("SPRT passed: {wins}-{draws}-{losses} ({elo:+.1} +/- {ci95:.1} Elo, LLR {llr:+.2} >= {upper:.2})");
        if auto_promote {
            sqlx::query("UPDATE gates SET status = 'promoted', decided_at = ?, decision_reason = ? WHERE id = ?")
                .bind(t)
                .bind(&reason)
                .bind(gate_id)
                .execute(&s.db)
                .await?;
            promote_network_tx(
                s,
                &candidate_sha,
                "coordinator",
                &format!("gate {gate_id} ({candidate_label}): {reason}"),
            )
            .await?;
            s.audit("coordinator", "gate-pass", &format!("gate {gate_id} ({candidate_label}): {reason}")).await;
        } else {
            sqlx::query("UPDATE gates SET status = 'passed', decided_at = ?, decision_reason = ? WHERE id = ?")
                .bind(t)
                .bind(&reason)
                .bind(gate_id)
                .execute(&s.db)
                .await?;
            s.audit("coordinator", "gate-pass", &format!("gate {gate_id} ({candidate_label}): {reason}")).await;
        }
    } else if llr <= lower {
        cancel_queued_units(s, round_id, "SPRT failed (early stop)").await?;
        let t = now() as i64;
        let reason =
            format!("SPRT failed: {wins}-{draws}-{losses} ({elo:+.1} +/- {ci95:.1} Elo, LLR {llr:+.2} <= {lower:.2})");
        sqlx::query("UPDATE gates SET status = 'failed', decided_at = ?, decision_reason = ? WHERE id = ?")
            .bind(t)
            .bind(&reason)
            .bind(gate_id)
            .execute(&s.db)
            .await?;
        sqlx::query("UPDATE networks SET status = 'rejected' WHERE sha256 = ? AND status = 'candidate'")
            .bind(&candidate_sha)
            .execute(&s.db)
            .await?;
        s.audit("coordinator", "gate-fail", &format!("gate {gate_id} ({candidate_label}): {reason}")).await;
    } else if pending == 0 && total_games > 0 {
        let t = now() as i64;
        let reason = format!("inconclusive after {total_games} games ({wins}-{draws}-{losses}, LLR {llr:+.2})");
        sqlx::query("UPDATE gates SET status = 'inconclusive', decided_at = ?, decision_reason = ? WHERE id = ?")
            .bind(t)
            .bind(&reason)
            .bind(gate_id)
            .execute(&s.db)
            .await?;
    }
    Ok(())
}

async fn cancel_queued_units(s: &Swarm, round_id: i64, reason: &str) -> Result<(), sqlx::Error> {
    let _one = s.lease.lock().await;
    sqlx::query(
        "UPDATE units SET state = 'cancelled', host = NULL, lease_until = NULL, verdict = ? \
         WHERE round = ? AND kind != 'verify' AND state = 'queued'",
    )
    .bind(reason)
    .bind(round_id)
    .execute(&s.db)
    .await?;
    Ok(())
}

/// Recomputes anchored Archbench Elo ratings from all `accepted` `UnitKind::Bench` units in a bench round.
pub(crate) async fn evaluate_bench(s: &Swarm, bench_id: i64) -> Res<()> {
    let row = sqlx::query("SELECT round_id, net_sha, anchor_elo FROM benches WHERE id = ?")
        .bind(bench_id)
        .fetch_optional(&s.db)
        .await?;
    let Some(row) = row else { return Ok(()) };
    let round_id: i64 = row.get("round_id");
    let net_sha: String = row.get("net_sha");
    let anchor_elo: f64 = row.get("anchor_elo");

    let urows = sqlx::query(
        "SELECT unit, manifest FROM units \
         WHERE round = ? AND kind = 'bench' AND state = 'accepted' AND manifest IS NOT NULL",
    )
    .bind(round_id)
    .fetch_all(&s.db)
    .await?;

    let mut per_opp: BTreeMap<String, (u64, u64, u64)> = BTreeMap::new();
    let (mut tw, mut td, mut tl) = (0u64, 0u64, 0u64);
    for ur in urows {
        let work: WorkUnit =
            serde_json::from_str(ur.get::<String, _>("unit").as_str()).map_err(|e| bad(e.to_string()))?;
        let manifest: ResultManifest =
            serde_json::from_str(ur.get::<String, _>("manifest").as_str()).map_err(|e| bad(e.to_string()))?;
        for g in &manifest.games {
            let opp_name = work
                .opponents
                .get(g.opponent as usize)
                .map_or_else(|| format!("opp#{}", g.opponent), |o| o.config.clone());
            let e = per_opp.entry(opp_name).or_insert((0, 0, 0));
            match g.winner {
                None => {
                    e.1 += 1;
                    td += 1;
                }
                Some(w) if w == g.learner => {
                    e.0 += 1;
                    tw += 1;
                }
                _ => {
                    e.2 += 1;
                    tl += 1;
                }
            }
        }
    }

    let games = tw + td + tl;
    let (elo_diff, ci95) = elo_and_ci95(tw, td, tl);
    let elo = anchor_elo + elo_diff;
    let anchors: Vec<AnchorScore> = per_opp
        .into_iter()
        .map(|(opponent, (w, d, l))| {
            let (de, ci) = elo_and_ci95(w, d, l);
            AnchorScore { opponent, wins: w, draws: d, losses: l, elo: anchor_elo + de, ci95: ci }
        })
        .collect();
    let anchors_json = serde_json::to_string(&anchors).unwrap_or_else(|_| "[]".into());
    let t = now() as i64;

    sqlx::query("UPDATE benches SET games = ?, elo = ?, ci95 = ?, anchors_json = ?, updated_at = ? WHERE id = ?")
        .bind(games as i64)
        .bind(elo)
        .bind(ci95)
        .bind(&anchors_json)
        .bind(t)
        .bind(bench_id)
        .execute(&s.db)
        .await?;
    sqlx::query(
        "UPDATE networks SET bench_elo = ?, bench_ci95 = ?, bench_games = ?, anchors_json = ? WHERE sha256 = ?",
    )
    .bind(elo)
    .bind(ci95)
    .bind(games as i64)
    .bind(&anchors_json)
    .bind(&net_sha)
    .execute(&s.db)
    .await?;
    Ok(())
}

// ---- HTTP handlers

fn gate_from_row(r: &sqlx::sqlite::SqliteRow) -> GateStatus {
    let alpha: f64 = r.get("alpha");
    let beta: f64 = r.get("beta");
    let (lower_bound, upper_bound) = sprt_bounds(alpha, beta);
    GateStatus {
        id: r.get("id"),
        round_id: r.get("round_id"),
        project: r.get("project"),
        run: r.get("run"),
        round: r.get::<i64, _>("round") as u32,
        candidate_sha: r.get("candidate_sha"),
        candidate_label: r.get("candidate_label"),
        baseline_sha: r.get("baseline_sha"),
        baseline_label: r.get("baseline_label"),
        status: r.get("status"),
        held: r.get::<i64, _>("held") != 0,
        auto_promote: r.get::<i64, _>("auto_promote") != 0,
        elo0: r.get("elo0"),
        elo1: r.get("elo1"),
        alpha,
        beta,
        lower_bound,
        upper_bound,
        wins: r.get::<i64, _>("wins") as u64,
        draws: r.get::<i64, _>("draws") as u64,
        losses: r.get::<i64, _>("losses") as u64,
        elo: r.get("elo"),
        elo_ci95: r.get("elo_ci95"),
        llr: r.get("llr"),
        max_share: r.get("max_share"),
        top_account_share: r.get("top_account_share"),
        created: r.get("created"),
        decided_at: r.get("decided_at"),
        decision_reason: r.get("decision_reason"),
    }
}

async fn load_gate(s: &Swarm, id: i64) -> Res<GateStatus> {
    let r = sqlx::query(
        "SELECT g.*, r.project, r.run, r.round FROM gates g JOIN rounds r ON r.id = g.round_id WHERE g.id = ?",
    )
    .bind(id)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no gate {id}")))?;
    Ok(gate_from_row(&r))
}

pub(crate) async fn new_gate(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(req): Json<NewGate>,
) -> Res<Json<GateStatus>> {
    let c = caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    if req.sprt.elo1 <= req.sprt.elo0 {
        return Err(bad("sprt.elo1 must be greater than sprt.elo0"));
    }
    if !(0.0 < req.sprt.alpha && req.sprt.alpha < 0.5 && 0.0 < req.sprt.beta && req.sprt.beta < 0.5) {
        return Err(bad("sprt.alpha and sprt.beta must be in (0, 0.5)"));
    }
    if !(0.0 < req.sprt.max_share && req.sprt.max_share <= 1.0) {
        return Err(bad("sprt.max_share must be in (0, 1]"));
    }
    let round_id =
        insert_typed_round(&s, &req.project, &req.run, req.round, req.priority, UnitKind::Match, req.units).await?;

    let base_label = if req.baseline_label.is_empty() {
        req.baseline_sha.chars().take(12).collect()
    } else {
        req.baseline_label.clone()
    };
    ensure_network(&s, &req.baseline_sha, &base_label, None, "promoted").await?;
    ensure_network(&s, &req.candidate_sha, &req.candidate_label, Some(&req.baseline_sha), "candidate").await?;

    let gid: i64 = sqlx::query(
        "INSERT INTO gates (round_id, candidate_sha, candidate_label, baseline_sha, baseline_label, status, \
         held, auto_promote, elo0, elo1, alpha, beta, max_share, created) \
         VALUES (?, ?, ?, ?, ?, 'running', 0, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(round_id)
    .bind(&req.candidate_sha)
    .bind(&req.candidate_label)
    .bind(&req.baseline_sha)
    .bind(&base_label)
    .bind(i64::from(req.sprt.auto_promote))
    .bind(req.sprt.elo0)
    .bind(req.sprt.elo1)
    .bind(req.sprt.alpha)
    .bind(req.sprt.beta)
    .bind(req.sprt.max_share)
    .bind(now() as i64)
    .fetch_one(&s.db)
    .await?
    .get("id");

    s.audit(
        &c.name,
        "round",
        &format!(
            "gate {gid}: {} vs {} (SPRT [{}, {}], max_share={:.0}%)",
            req.candidate_label,
            base_label,
            req.sprt.elo0,
            req.sprt.elo1,
            req.sprt.max_share * 100.0
        ),
    )
    .await;
    load_gate(&s, gid).await.map(Json)
}

pub(crate) async fn list_gates(State(s): State<Shared>) -> Res<Json<Vec<GateStatus>>> {
    let rows = sqlx::query(
        "SELECT g.*, r.project, r.run, r.round FROM gates g JOIN rounds r ON r.id = g.round_id \
         ORDER BY g.id DESC LIMIT 100",
    )
    .fetch_all(&s.db)
    .await?;
    Ok(Json(rows.iter().map(gate_from_row).collect()))
}

pub(crate) async fn get_gate(State(s): State<Shared>, Path(id): Path<i64>) -> Res<Json<GateStatus>> {
    load_gate(&s, id).await.map(Json)
}

pub(crate) async fn hold_gate(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<GateAction>,
) -> Res<Json<GateStatus>> {
    let c = caller(&s, &headers, &[Role::Admin]).await?;
    let n = sqlx::query("UPDATE gates SET held = 1 WHERE id = ?").bind(id).execute(&s.db).await?.rows_affected();
    if n == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, format!("no gate {id}")));
    }
    s.audit(&c.name, "gate-hold", &format!("gate {id} held: {}", req.reason)).await;
    load_gate(&s, id).await.map(Json)
}

pub(crate) async fn resume_gate(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<GateAction>,
) -> Res<Json<GateStatus>> {
    let c = caller(&s, &headers, &[Role::Admin]).await?;
    let n = sqlx::query("UPDATE gates SET held = 0 WHERE id = ?").bind(id).execute(&s.db).await?.rows_affected();
    if n == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, format!("no gate {id}")));
    }
    s.audit(&c.name, "gate-hold", &format!("gate {id} resumed: {}", req.reason)).await;
    evaluate_gate(&s, id).await?;
    load_gate(&s, id).await.map(Json)
}

pub(crate) async fn promote_gate(
    State(s): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<GateAction>,
) -> Res<Json<GateStatus>> {
    let c = caller(&s, &headers, &[Role::Admin]).await?;
    let g = load_gate(&s, id).await?;
    let passed = matches!(g.status.as_str(), "passed" | "held");
    if !passed && req.reason.trim().is_empty() {
        return Err(bad("promoting a gate that has not passed requires a written reason"));
    }
    let reason = if req.reason.trim().is_empty() {
        format!("manual promotion of passed gate {id}")
    } else {
        req.reason.trim().to_string()
    };
    sqlx::query("UPDATE gates SET status = 'promoted', held = 0, decided_at = ?, decision_reason = ? WHERE id = ?")
        .bind(now() as i64)
        .bind(&reason)
        .bind(id)
        .execute(&s.db)
        .await?;
    cancel_queued_units(&s, g.round_id, "gate manually promoted").await?;
    promote_network_tx(&s, &g.candidate_sha, &c.name, &format!("gate {id} ({}): {reason}", g.candidate_label)).await?;
    load_gate(&s, id).await.map(Json)
}

pub(crate) async fn rollback_network(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(req): Json<RollbackRequest>,
) -> Res<StatusCode> {
    let c = caller(&s, &headers, &[Role::Admin]).await?;
    if req.reason.trim().is_empty() {
        return Err(bad("rolling back a network requires a written reason"));
    }
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM networks WHERE sha256 = ?")
        .bind(&req.target_sha)
        .fetch_one(&s.db)
        .await?;
    if exists == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, format!("no network {}", req.target_sha)));
    }
    let t = now() as i64;
    sqlx::query("UPDATE networks SET status = 'rolled-back' WHERE status = 'promoted' AND sha256 != ?")
        .bind(&req.target_sha)
        .execute(&s.db)
        .await?;
    sqlx::query("UPDATE gates SET status = 'rolled-back' WHERE status = 'promoted' AND candidate_sha != ?")
        .bind(&req.target_sha)
        .execute(&s.db)
        .await?;
    sqlx::query("UPDATE networks SET status = 'promoted', promoted_at = ? WHERE sha256 = ?")
        .bind(t)
        .bind(&req.target_sha)
        .execute(&s.db)
        .await?;
    s.audit(&c.name, "rollback", &format!("rolled back to {}: {}", req.target_sha, req.reason.trim())).await;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn new_bench(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(req): Json<NewBench>,
) -> Res<Json<BenchStatus>> {
    let c = caller(&s, &headers, &[Role::Trainer, Role::Admin]).await?;
    let round_id =
        insert_typed_round(&s, &req.project, &req.run, req.round, req.priority, UnitKind::Bench, req.units).await?;
    ensure_network(&s, &req.net_sha, &req.net_label, None, "candidate").await?;
    let t = now() as i64;
    let bid: i64 = sqlx::query(
        "INSERT INTO benches (round_id, net_sha, net_label, anchor_elo, elo, created, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(round_id)
    .bind(&req.net_sha)
    .bind(&req.net_label)
    .bind(req.anchor_elo)
    .bind(req.anchor_elo)
    .bind(t)
    .bind(t)
    .fetch_one(&s.db)
    .await?
    .get("id");
    s.audit(&c.name, "bench", &format!("bench {bid} for {} (anchor Elo {:.0})", req.net_label, req.anchor_elo)).await;
    load_bench(&s, bid).await.map(Json)
}

async fn load_bench(s: &Swarm, id: i64) -> Res<BenchStatus> {
    let r = sqlx::query(
        "SELECT b.*, r.project, r.run, r.round FROM benches b JOIN rounds r ON r.id = b.round_id WHERE b.id = ?",
    )
    .bind(id)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no bench {id}")))?;
    Ok(bench_from_row(&r))
}

fn bench_from_row(r: &sqlx::sqlite::SqliteRow) -> BenchStatus {
    let anchors: Vec<AnchorScore> =
        serde_json::from_str(r.get::<String, _>("anchors_json").as_str()).unwrap_or_default();
    BenchStatus {
        id: r.get("id"),
        round_id: r.get("round_id"),
        project: r.get("project"),
        run: r.get("run"),
        round: r.get::<i64, _>("round") as u32,
        net_sha: r.get("net_sha"),
        net_label: r.get("net_label"),
        anchor_elo: r.get("anchor_elo"),
        games: r.get::<i64, _>("games") as u64,
        elo: r.get("elo"),
        ci95: r.get("ci95"),
        anchors,
        created: r.get("created"),
        updated_at: r.get("updated_at"),
    }
}

pub(crate) async fn list_benches(State(s): State<Shared>) -> Res<Json<Vec<BenchStatus>>> {
    let rows = sqlx::query(
        "SELECT b.*, r.project, r.run, r.round FROM benches b JOIN rounds r ON r.id = b.round_id \
         ORDER BY b.id DESC LIMIT 100",
    )
    .fetch_all(&s.db)
    .await?;
    Ok(Json(rows.iter().map(bench_from_row).collect()))
}

pub(crate) async fn list_networks(State(s): State<Shared>) -> Res<Json<Vec<NetworkInfo>>> {
    let rows = sqlx::query(
        "SELECT n.sha256, n.label, n.parent_sha, n.status, n.created, n.promoted_at, \
         n.bench_elo, n.bench_ci95, n.bench_games, n.anchors_json, \
         (SELECT id FROM gates g WHERE g.candidate_sha = n.sha256 ORDER BY g.id DESC LIMIT 1) AS gate_id, \
         (SELECT status FROM gates g WHERE g.candidate_sha = n.sha256 ORDER BY g.id DESC LIMIT 1) AS gate_status, \
         (SELECT elo FROM gates g WHERE g.candidate_sha = n.sha256 ORDER BY g.id DESC LIMIT 1) AS gate_elo, \
         (SELECT llr FROM gates g WHERE g.candidate_sha = n.sha256 ORDER BY g.id DESC LIMIT 1) AS gate_llr \
         FROM networks n ORDER BY n.created ASC, n.sha256 ASC",
    )
    .fetch_all(&s.db)
    .await?;

    let out = rows
        .into_iter()
        .map(|r| {
            let sha: String = r.get("sha256");
            let anchors: Vec<AnchorScore> =
                serde_json::from_str(r.get::<String, _>("anchors_json").as_str()).unwrap_or_default();
            NetworkInfo {
                download_url: if valid_sha(&sha) { format!("{}/api/v1/nets/{sha}", s.base) } else { String::new() },
                sha256: sha,
                label: r.get("label"),
                parent_sha: r.get("parent_sha"),
                status: r.get("status"),
                created: r.get("created"),
                promoted_at: r.get("promoted_at"),
                gate_id: r.get("gate_id"),
                gate_status: r.get("gate_status"),
                gate_elo: r.get("gate_elo"),
                gate_llr: r.get("gate_llr"),
                bench_elo: r.get("bench_elo"),
                bench_ci95: r.get("bench_ci95"),
                bench_games: r.get::<i64, _>("bench_games") as u64,
                anchors,
            }
        })
        .collect();
    Ok(Json(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprt_bounds_and_llr_behave_as_expected() {
        let (lo, hi) = sprt_bounds(0.05, 0.05);
        assert!((lo + 2.944439).abs() < 1e-4);
        assert!((hi - 2.944439).abs() < 1e-4);

        // Clear winner (~60% score) crosses upper bound within 400 games
        let llr_win = sprt_llr(235, 10, 155, 0.0, 15.0);
        assert!(llr_win > hi, "expected LLR {llr_win} > {hi}");

        // Clear loser crosses lower bound
        let llr_lose = sprt_llr(155, 10, 235, 0.0, 15.0);
        assert!(llr_lose < lo, "expected LLR {llr_lose} < {lo}");

        // Exact 50-50 stays near 0 / slightly negative against positive elo1
        let llr_even = sprt_llr(50, 0, 50, 0.0, 10.0);
        assert!(llr_even < 0.0 && llr_even > lo);
    }

    #[test]
    fn elo_and_ci95_matches_logistic_scale() {
        let (elo, ci) = elo_and_ci95(50, 0, 50);
        assert!(elo.abs() < 1e-6);
        assert!(ci > 50.0 && ci < 80.0);
        let (elo2, _) = elo_and_ci95(75, 0, 25);
        assert!((elo2 - 190.85).abs() < 0.5, "got {elo2}");
    }
}
