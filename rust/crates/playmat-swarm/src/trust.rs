//! Mana@Home S2 trust, statistics, quarantine and credit (docs/swarm/Design.md).
//!
//! - Trust tiers: Probation (100% re-run on first 20 units per account and per host) -> Normal (3% re-run) ->
//!   Reputable (30 days + 2,000 accepted units with 0 mismatches; 3% self-play, 10% gate/bench; eligible for verify
//!   and gate units) -> Trusted (0% re-run; runs verify/gate/bench).
//! - Per-worker statistics: compares each worker's recent units against peer workers on win rate, game length,
//!   policy-target entropy, and (suspiciously fast) time per game. Outliers open a `stat-flag` inbox item and bump
//!   the account to 100% re-run audits rather than auto-banning, so fast honest hardware passes re-runs cleanly.
//! - Credits: Folding@home style base credit (`unit.cost`) times a capped quick-return bonus (`sqrt(window/elapsed)`,
//!   capped at 1.5x). Credits start `pending` and mature 7 days after acceptance. On a verification mismatch, the
//!   unit earns nothing, the account's untrained units are quarantined, its last 30 days of credit are frozen, and an
//!   item opens in the admin verification inbox.

use crate::store::now;
use playmat_swarm_proto::api::TrustLevel;
use playmat_swarm_proto::{ResultManifest, UnitKind, VerifyMode};
use sqlx::{Row, SqlitePool};

pub const PROBATION_UNITS: i64 = 20;
pub const REPUTABLE_UNITS: i64 = 2_000;
pub const REPUTABLE_AGE_SECS: i64 = 30 * 86_400;
pub const NORMAL_AUDIT_PCT: u32 = 3;
pub const GATE_AUDIT_PCT: u32 = 10;
pub const CREDIT_MATURE_SECS: i64 = 7 * 86_400;
pub const CREDIT_FREEZE_WINDOW_SECS: i64 = 30 * 86_400;
pub const EXTRA_AUDIT_SECS: i64 = 7 * 86_400;
pub const MAX_QRB_MULT: f64 = 1.5;

const MIN_WORKER_UNITS: usize = 3;
const MIN_PEER_UNITS: usize = 5;
const Z_SINGLE_THRESHOLD: f64 = 4.0;
const CHI2_3DF_THRESHOLD: f64 = 21.1; // p < 1e-4
const CHI2_4DF_THRESHOLD: f64 = 23.5; // p < 1e-4

/// Folding@home-style quick-return bonus: `sqrt(window / elapsed)` capped at `MAX_QRB_MULT` (1.5x).
pub fn quick_return_multiplier(elapsed_secs: u64, window_secs: u64) -> f64 {
    if window_secs == 0 || elapsed_secs >= window_secs {
        return 1.0;
    }
    let ratio = window_secs as f64 / elapsed_secs.max(1) as f64;
    ratio.sqrt().clamp(1.0, MAX_QRB_MULT)
}

/// Computes `(base, bonus_mult, total)` credit for an accepted unit.
pub fn compute_credit(cost: f64, elapsed_secs: u64, window_secs: u64) -> (f64, f64, f64) {
    let base = cost.max(0.0);
    let mult = quick_return_multiplier(elapsed_secs, window_secs);
    (base, mult, base * mult)
}

/// Unpredictable audit roll in `0..100` using OS randomness so workers cannot predict which units get audited.
pub fn random_roll() -> u32 {
    let mut b = [0u8; 4];
    if getrandom::getrandom(&mut b).is_ok() {
        u32::from_le_bytes(b) % 100
    } else {
        // fallback: audit conservatively
        0
    }
}

/// Decides whether a completed unit needs a full re-run audit (instead of replay-only).
pub fn should_rerun(
    trust: TrustLevel,
    host_probation_passed: bool,
    flagged: bool,
    kind: UnitKind,
    roll_0_99: u32,
) -> bool {
    // GPU-batched units are non-bit-exact across hardware/precision and are verified via deterministic
    // answer-log Replay plus statistical residual checks rather than CPU bit-exact re-runs.
    if kind == UnitKind::SelfplayGpu {
        return false;
    }
    match trust {
        TrustLevel::Trusted => false,
        TrustLevel::Probation | TrustLevel::Banned => true,
        TrustLevel::Normal => !host_probation_passed || flagged || roll_0_99 < NORMAL_AUDIT_PCT,
        TrustLevel::Reputable => {
            if !host_probation_passed || flagged {
                return true;
            }
            let pct = match kind {
                UnitKind::Match | UnitKind::Bench => GATE_AUDIT_PCT,
                _ => NORMAL_AUDIT_PCT,
            };
            roll_0_99 < pct
        }
    }
}

pub fn unit_kind_name(k: UnitKind) -> &'static str {
    match k {
        UnitKind::Selfplay => "selfplay",
        UnitKind::SelfplayGpu => "selfplay-gpu",
        UnitKind::Match => "match",
        UnitKind::Bench => "bench",
        UnitKind::Verify => "verify",
    }
}

pub fn unit_kind_parse(s: &str) -> UnitKind {
    match s {
        "selfplay-gpu" => UnitKind::SelfplayGpu,
        "match" => UnitKind::Match,
        "bench" => UnitKind::Bench,
        "verify" => UnitKind::Verify,
        _ => UnitKind::Selfplay,
    }
}

/// Summary extracted from a completed `ResultManifest` for per-worker statistical checks.
#[derive(Clone, Debug)]
pub struct UnitMetrics {
    pub games: i64,
    pub learner_wins: i64,
    pub draws: i64,
    pub total_turns: i64,
    pub total_answers: i64,
    pub total_recorded: i64,
    pub mean_entropy: Option<f64>,
    pub ms_per_game: f64,
}

pub fn extract_metrics(m: &ResultManifest) -> UnitMetrics {
    let games = m.games.len() as i64;
    let mut learner_wins = 0i64;
    let mut draws = 0i64;
    let mut total_turns = 0i64;
    let mut total_answers = 0i64;
    let mut total_recorded = 0i64;
    let mut ent_sum = 0.0f64;
    let mut ent_n = 0u64;
    for g in &m.games {
        match g.winner {
            None => draws += 1,
            Some(w) if w == g.learner => learner_wins += 1,
            _ => {}
        }
        total_turns += g.turns as i64;
        total_answers += g.answers as i64;
        total_recorded += g.recorded as i64;
        if let Some(e) = g.policy_entropy {
            if e.is_finite() {
                ent_sum += e as f64;
                ent_n += 1;
            }
        }
    }
    let mean_entropy = if ent_n > 0 { Some(ent_sum / ent_n as f64) } else { None };
    let ms_per_game = if games > 0 { m.elapsed_ms as f64 / games as f64 } else { 0.0 };
    UnitMetrics { games, learner_wins, draws, total_turns, total_answers, total_recorded, mean_entropy, ms_per_game }
}

/// Records a unit's metrics in `unit_stats`.
pub async fn record_unit_stats(
    db: &SqlitePool,
    unit_id: &str,
    round: i64,
    token: i64,
    host: i64,
    kind: UnitKind,
    m: &ResultManifest,
) -> Result<(), sqlx::Error> {
    let met = extract_metrics(m);
    sqlx::query(
        "INSERT OR REPLACE INTO unit_stats \
         (unit, round, token, host, kind, games, learner_wins, draws, total_turns, total_answers, total_recorded, \
          mean_entropy, ms_per_game, created) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(unit_id)
    .bind(round)
    .bind(token)
    .bind(host)
    .bind(unit_kind_name(kind))
    .bind(met.games)
    .bind(met.learner_wins)
    .bind(met.draws)
    .bind(met.total_turns)
    .bind(met.total_answers)
    .bind(met.total_recorded)
    .bind(met.mean_entropy)
    .bind(met.ms_per_game)
    .bind(now() as i64)
    .execute(db)
    .await?;
    Ok(())
}

#[derive(Clone, Debug)]
struct StatRow {
    games: f64,
    wins: f64,
    draws: f64,
    turns_per_game: f64,
    mean_entropy: Option<f64>,
    ms_per_game: f64,
}

#[derive(Clone, Debug)]
pub struct StatFlag {
    pub detail: String,
}

fn mean_std(vals: &[f64], floor: f64) -> (f64, f64) {
    let n = vals.len() as f64;
    let mean = vals.iter().sum::<f64>() / n;
    let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
    (mean, var.sqrt().max(floor))
}

/// Evaluates statistical residuals for a worker's recent units against peers.
fn evaluate_residuals(worker: &[StatRow], peers: &[StatRow]) -> Option<StatFlag> {
    if worker.len() < MIN_WORKER_UNITS || peers.len() < MIN_PEER_UNITS {
        return None;
    }
    let w_games: f64 = worker.iter().map(|r| r.games).sum();
    let p_games: f64 = peers.iter().map(|r| r.games).sum();
    if w_games < 6.0 || p_games < 10.0 {
        return None;
    }

    // 1. Win rate residual
    let w_wr = worker.iter().map(|r| r.wins + 0.5 * r.draws).sum::<f64>() / w_games;
    let p_wr = peers.iter().map(|r| r.wins + 0.5 * r.draws).sum::<f64>() / p_games;
    let wr_se = (p_wr * (1.0 - p_wr) / w_games + 1e-4).sqrt();
    let z_wr = (w_wr - p_wr) / wr_se;

    // 2. Game length (turns/game) residual
    let w_turns: Vec<f64> = worker.iter().map(|r| r.turns_per_game).collect();
    let p_turns: Vec<f64> = peers.iter().map(|r| r.turns_per_game).collect();
    let (p_t_mean, p_t_std) = mean_std(&p_turns, 1.5);
    let w_t_mean = w_turns.iter().sum::<f64>() / w_turns.len() as f64;
    let z_turns = (w_t_mean - p_t_mean) / (p_t_std / (w_turns.len() as f64).sqrt());

    // 3. Speed (log ms/game): only suspiciously FAST workers flag (negative z)
    let w_logms: Vec<f64> = worker.iter().map(|r| r.ms_per_game.max(1.0).ln()).collect();
    let p_logms: Vec<f64> = peers.iter().map(|r| r.ms_per_game.max(1.0).ln()).collect();
    let (p_s_mean, p_s_std) = mean_std(&p_logms, 0.5);
    let w_s_mean = w_logms.iter().sum::<f64>() / w_logms.len() as f64;
    let z_speed = (w_s_mean - p_s_mean) / (p_s_std / (w_logms.len() as f64).sqrt());
    let z_speed_fast = z_speed.min(0.0);

    // 4. Policy target entropy residual (optional, if reported)
    let w_ent: Vec<f64> = worker.iter().filter_map(|r| r.mean_entropy).collect();
    let p_ent: Vec<f64> = peers.iter().filter_map(|r| r.mean_entropy).collect();
    let z_ent = if w_ent.len() >= MIN_WORKER_UNITS && p_ent.len() >= MIN_PEER_UNITS {
        let (p_e_mean, p_e_std) = mean_std(&p_ent, 0.08);
        let w_e_mean = w_ent.iter().sum::<f64>() / w_ent.len() as f64;
        Some((w_e_mean - p_e_mean) / (p_e_std / (w_ent.len() as f64).sqrt()))
    } else {
        None
    };

    let (chi2, thresh) = match z_ent {
        Some(ze) => (z_wr * z_wr + z_turns * z_turns + z_speed_fast * z_speed_fast + ze * ze, CHI2_4DF_THRESHOLD),
        None => (z_wr * z_wr + z_turns * z_turns + z_speed_fast * z_speed_fast, CHI2_3DF_THRESHOLD),
    };

    let triggered = z_wr.abs() >= Z_SINGLE_THRESHOLD
        || z_turns.abs() >= Z_SINGLE_THRESHOLD
        || z_speed <= -Z_SINGLE_THRESHOLD
        || z_ent.is_some_and(|ze| ze.abs() >= Z_SINGLE_THRESHOLD)
        || chi2 >= thresh;

    if !triggered {
        return None;
    }

    let mut parts = vec![
        format!("win_rate={w_wr:.3} vs peers {p_wr:.3} (z={z_wr:+.2})"),
        format!("turns={w_t_mean:.2} vs {p_t_mean:.2} (z={z_turns:+.2})"),
        format!("speed_z={z_speed:+.2}"),
    ];
    if let Some(ze) = z_ent {
        parts.push(format!("policy_entropy_z={ze:+.2}"));
    }
    parts.push(format!("chi2={chi2:.1} (n={} units, peers={})", worker.len(), peers.len()));
    Some(StatFlag { detail: parts.join(", ") })
}

async fn load_stat_rows(rows: Vec<sqlx::sqlite::SqliteRow>) -> Vec<StatRow> {
    rows.into_iter()
        .filter_map(|r| {
            let games = r.get::<i64, _>("games") as f64;
            if games <= 0.0 {
                return None;
            }
            Some(StatRow {
                games,
                wins: r.get::<i64, _>("learner_wins") as f64,
                draws: r.get::<i64, _>("draws") as f64,
                turns_per_game: r.get::<i64, _>("total_turns") as f64 / games,
                mean_entropy: r.get::<Option<f64>, _>("mean_entropy"),
                ms_per_game: r.get::<f64, _>("ms_per_game"),
            })
        })
        .collect()
}

/// Checks per-worker statistics against peers on the same round/kind. If anomalous, bumps `extra_audit_until`
/// to force 100% re-runs and opens a `stat-flag` inbox item (deduplicated while one is already open).
pub async fn check_worker_stats(
    db: &SqlitePool,
    round: i64,
    token: i64,
    host: i64,
    kind: UnitKind,
    unit_id: &str,
) -> Result<Option<StatFlag>, sqlx::Error> {
    let kind_s = unit_kind_name(kind);
    let worker_rows = load_stat_rows(
        sqlx::query(
            "SELECT s.games, s.learner_wins, s.draws, s.total_turns, s.mean_entropy, s.ms_per_game \
             FROM unit_stats s JOIN units u ON u.id = s.unit \
             WHERE s.token = ? AND s.kind = ? AND u.state IN ('complete', 'accepted') \
             ORDER BY s.created DESC LIMIT 20",
        )
        .bind(token)
        .bind(kind_s)
        .fetch_all(db)
        .await?,
    )
    .await;
    if worker_rows.len() < MIN_WORKER_UNITS {
        return Ok(None);
    }

    let mut peer_rows = load_stat_rows(
        sqlx::query(
            "SELECT s.games, s.learner_wins, s.draws, s.total_turns, s.mean_entropy, s.ms_per_game \
             FROM unit_stats s JOIN units u ON u.id = s.unit \
             WHERE s.round = ? AND s.kind = ? AND s.token != ? AND u.state = 'accepted' \
             ORDER BY s.created DESC LIMIT 200",
        )
        .bind(round)
        .bind(kind_s)
        .bind(token)
        .fetch_all(db)
        .await?,
    )
    .await;
    if peer_rows.len() < MIN_PEER_UNITS {
        peer_rows = load_stat_rows(
            sqlx::query(
                "SELECT s.games, s.learner_wins, s.draws, s.total_turns, s.mean_entropy, s.ms_per_game \
                 FROM unit_stats s JOIN units u ON u.id = s.unit \
                 WHERE s.kind = ? AND s.token != ? AND u.state = 'accepted' \
                 ORDER BY s.created DESC LIMIT 200",
            )
            .bind(kind_s)
            .bind(token)
            .fetch_all(db)
            .await?,
        )
        .await;
    }

    let Some(flag) = evaluate_residuals(&worker_rows, &peer_rows) else {
        return Ok(None);
    };

    let t = now() as i64;
    sqlx::query("UPDATE tokens SET extra_audit_until = MAX(extra_audit_until, ?) WHERE id = ?")
        .bind(t + EXTRA_AUDIT_SECS)
        .bind(token)
        .execute(db)
        .await?;

    // avoid flooding the inbox if an open stat-flag already exists for this account
    let open: i64 =
        sqlx::query("SELECT COUNT(*) AS n FROM inbox WHERE status = 'open' AND kind = 'stat-flag' AND token = ?")
            .bind(token)
            .fetch_one(db)
            .await?
            .get("n");
    if open == 0 {
        sqlx::query(
            "INSERT INTO inbox (created, status, kind, token, host, unit, detail) \
             VALUES (?, 'open', 'stat-flag', ?, ?, ?, ?)",
        )
        .bind(t)
        .bind(token)
        .bind(host)
        .bind(unit_id)
        .bind(&flag.detail)
        .execute(db)
        .await?;
    }
    Ok(Some(flag))
}

/// Promotes any pending credits whose 7-day maturity window has elapsed.
pub async fn mature_credits(db: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE credits SET status = 'matured' WHERE status = 'pending' AND mature_at <= ?")
        .bind(now() as i64)
        .execute(db)
        .await?;
    Ok(())
}

pub struct CreditGrant<'a> {
    pub token: i64,
    pub host: Option<i64>,
    pub unit_id: &'a str,
    pub kind: &'a str,
    pub cost: f64,
    pub elapsed_secs: u64,
    pub window_secs: u64,
}

/// Awards Folding@home-style credit (base * quick-return bonus) for an accepted unit.
pub async fn award_credit(db: &SqlitePool, g: CreditGrant<'_>) -> Result<f64, sqlx::Error> {
    let t = now() as i64;
    let (base, mult, credit) = compute_credit(g.cost, g.elapsed_secs, g.window_secs);
    let mature_at = t + CREDIT_MATURE_SECS;
    sqlx::query(
        "INSERT INTO credits (at, token, host, unit, kind, base_credit, bonus_mult, credit, status, mature_at, reason) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, '')",
    )
    .bind(t)
    .bind(g.token)
    .bind(g.host)
    .bind(g.unit_id)
    .bind(g.kind)
    .bind(base)
    .bind(mult)
    .bind(credit)
    .bind(mature_at)
    .execute(db)
    .await?;
    sqlx::query("UPDATE units SET credit = ?, credit_status = 'pending', credit_mature_at = ? WHERE id = ?")
        .bind(credit)
        .bind(mature_at)
        .bind(g.unit_id)
        .execute(db)
        .await?;
    Ok(credit)
}

/// Increments accepted counts for `(token, host)` and promotes trust tiers when thresholds are reached.
/// Returns `Some(new_trust)` if the account was promoted.
pub async fn record_acceptance(
    db: &SqlitePool,
    token: i64,
    host: i64,
    count_for_probation: bool,
) -> Result<Option<TrustLevel>, sqlx::Error> {
    if !count_for_probation {
        return Ok(None);
    }
    sqlx::query("UPDATE hosts SET accepted_units = accepted_units + 1 WHERE id = ?").bind(host).execute(db).await?;
    sqlx::query("UPDATE tokens SET accepted_units = accepted_units + 1 WHERE id = ?").bind(token).execute(db).await?;

    let h = sqlx::query("SELECT accepted_units, mismatches FROM hosts WHERE id = ?").bind(host).fetch_one(db).await?;
    if h.get::<i64, _>("accepted_units") >= PROBATION_UNITS && h.get::<i64, _>("mismatches") == 0 {
        sqlx::query("UPDATE hosts SET probation_passed = 1 WHERE id = ?").bind(host).execute(db).await?;
    }

    let row = sqlx::query("SELECT trust, created, accepted_units, mismatches FROM tokens WHERE id = ?")
        .bind(token)
        .fetch_one(db)
        .await?;
    let current = TrustLevel::parse(row.get::<String, _>("trust").as_str()).unwrap_or(TrustLevel::Probation);
    let accepted = row.get::<i64, _>("accepted_units");
    let mismatches = row.get::<i64, _>("mismatches");
    let age = now() as i64 - row.get::<i64, _>("created");

    let next = match current {
        TrustLevel::Probation if accepted >= PROBATION_UNITS && mismatches == 0 => Some(TrustLevel::Normal),
        TrustLevel::Normal if accepted >= REPUTABLE_UNITS && mismatches == 0 && age >= REPUTABLE_AGE_SECS => {
            Some(TrustLevel::Reputable)
        }
        _ => None,
    };
    if let Some(t) = next {
        sqlx::query("UPDATE tokens SET trust = ? WHERE id = ?").bind(t.name()).bind(token).execute(db).await?;
    }
    Ok(next)
}

/// Quarantines untrained data and freezes the last 30 days of credit for an account that failed verification.
/// Opens an admin inbox item with full details (including any already-trained rounds that need rollback review).
pub async fn handle_mismatch(
    db: &SqlitePool,
    source_token: i64,
    source_host: i64,
    target_unit: &str,
    verify_unit: Option<&str>,
    mode: VerifyMode,
    reason: &str,
) -> Result<(u32, u32), sqlx::Error> {
    let t = now() as i64;
    sqlx::query("UPDATE units SET state = 'rejected', quarantined = 1, verdict = ? WHERE id = ?")
        .bind(reason)
        .bind(target_unit)
        .execute(db)
        .await?;

    sqlx::query("UPDATE hosts SET mismatches = mismatches + 1, probation_passed = 0 WHERE id = ?")
        .bind(source_host)
        .execute(db)
        .await?;
    sqlx::query(
        "UPDATE tokens SET mismatches = mismatches + 1, \
         trust = CASE WHEN trust = 'banned' THEN 'banned' ELSE 'probation' END, \
         extra_audit_until = MAX(extra_audit_until, ?), \
         credit_frozen_until = MAX(credit_frozen_until, ?) \
         WHERE id = ?",
    )
    .bind(t + CREDIT_FREEZE_WINDOW_SECS)
    .bind(t + CREDIT_FREEZE_WINDOW_SECS)
    .bind(source_token)
    .execute(db)
    .await?;

    // quarantine all untrained units from this account (any round with trained = 0)
    let quarantined_units = sqlx::query(
        "UPDATE units SET state = 'quarantined', quarantined = 1 \
         WHERE id IN ( \
             SELECT u.id FROM units u \
             JOIN hosts h ON h.id = u.host \
             JOIN rounds r ON r.id = u.round \
             WHERE h.token = ? AND r.trained = 0 AND u.kind != 'verify' AND u.state IN ('complete', 'accepted') \
         )",
    )
    .bind(source_token)
    .execute(db)
    .await?
    .rows_affected() as u32;

    // cancel any still-queued verify units for those newly quarantined units
    sqlx::query(
        "UPDATE units SET state = 'cancelled', host = NULL, lease_until = NULL \
         WHERE kind = 'verify' AND state = 'queued' \
         AND source_unit IN (SELECT id FROM units WHERE state = 'quarantined')",
    )
    .execute(db)
    .await?;

    // freeze the last 30 days of credit for this account
    let frozen_credits = sqlx::query(
        "UPDATE credits SET status = 'frozen' \
         WHERE token = ? AND at >= ? AND status IN ('pending', 'matured')",
    )
    .bind(source_token)
    .bind(t - CREDIT_FREEZE_WINDOW_SECS)
    .execute(db)
    .await?
    .rows_affected() as u32;
    sqlx::query(
        "UPDATE units SET credit_status = 'frozen' \
         WHERE host IN (SELECT id FROM hosts WHERE token = ?) AND done_at >= ? AND credit_status IN ('pending', 'matured')",
    )
    .bind(source_token)
    .bind(t - CREDIT_FREEZE_WINDOW_SECS)
    .execute(db)
    .await?;

    // check if any already-trained rounds contain accepted units from this account (for admin rollback review)
    let trained_rounds: Vec<i64> = sqlx::query(
        "SELECT DISTINCT u.round FROM units u JOIN hosts h ON h.id = u.host JOIN rounds r ON r.id = u.round \
         WHERE h.token = ? AND r.trained = 1 AND u.kind != 'verify' AND u.state = 'accepted' ORDER BY u.round",
    )
    .bind(source_token)
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|r| r.get("round"))
    .collect();

    let kind = match mode {
        VerifyMode::Replay => "replay-fail",
        VerifyMode::Rerun => "rerun-mismatch",
    };
    let mut detail = format!(
        "{reason}; quarantined {quarantined_units} untrained unit(s), froze {frozen_credits} credit entry(ies)"
    );
    if !trained_rounds.is_empty() {
        detail.push_str(&format!("; already-trained rounds affected (review rollback): {trained_rounds:?}"));
    }

    sqlx::query(
        "INSERT INTO inbox (created, status, kind, token, host, unit, verify_unit, detail) \
         VALUES (?, 'open', ?, ?, ?, ?, ?, ?)",
    )
    .bind(t)
    .bind(kind)
    .bind(source_token)
    .bind(source_host)
    .bind(target_unit)
    .bind(verify_unit)
    .bind(&detail)
    .execute(db)
    .await?;

    Ok((quarantined_units, frozen_credits))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_return_bonus_follows_sqrt_curve_with_cap() {
        assert!((quick_return_multiplier(900, 900) - 1.0).abs() < 1e-9);
        assert!((quick_return_multiplier(1200, 900) - 1.0).abs() < 1e-9);
        // 450s of 900s -> sqrt(2) = 1.4142
        assert!((quick_return_multiplier(450, 900) - 2.0f64.sqrt()).abs() < 1e-6);
        // 100s of 900s -> sqrt(9) = 3.0, capped at 1.5
        assert!((quick_return_multiplier(100, 900) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn audit_sampling_matches_trust_tiers() {
        assert!(!should_rerun(TrustLevel::Trusted, true, false, UnitKind::Selfplay, 0));
        assert!(!should_rerun(TrustLevel::Trusted, true, true, UnitKind::Match, 0));
        assert!(should_rerun(TrustLevel::Probation, false, false, UnitKind::Selfplay, 99));
        // new host on a Normal account is still in host probation -> 100%
        assert!(should_rerun(TrustLevel::Normal, false, false, UnitKind::Selfplay, 99));
        // Normal after probation: 3%
        assert!(should_rerun(TrustLevel::Normal, true, false, UnitKind::Selfplay, 2));
        assert!(!should_rerun(TrustLevel::Normal, true, false, UnitKind::Selfplay, 3));
        // flagged Normal -> 100%
        assert!(should_rerun(TrustLevel::Normal, true, true, UnitKind::Selfplay, 99));
        // Reputable: 3% on selfplay, 10% on match/bench
        assert!(!should_rerun(TrustLevel::Reputable, true, false, UnitKind::Selfplay, 5));
        assert!(should_rerun(TrustLevel::Reputable, true, false, UnitKind::Match, 9));
        assert!(!should_rerun(TrustLevel::Reputable, true, false, UnitKind::Match, 10));
    }

    #[test]
    fn stats_residual_flags_cheater_not_honest_peers() {
        let peers: Vec<StatRow> = (0..10)
            .map(|i| StatRow {
                games: 50.0,
                wins: 25.0 + (i % 3) as f64 - 1.0,
                draws: 2.0,
                turns_per_game: 12.0 + (i % 3) as f64 * 0.5,
                mean_entropy: Some(0.85 + (i % 3) as f64 * 0.03),
                ms_per_game: 5000.0 + (i % 3) as f64 * 400.0,
            })
            .collect();
        // honest worker looks like peers -> no flag
        let honest = vec![
            StatRow {
                games: 50.0,
                wins: 26.0,
                draws: 2.0,
                turns_per_game: 12.4,
                mean_entropy: Some(0.87),
                ms_per_game: 4800.0,
            },
            StatRow {
                games: 50.0,
                wins: 24.0,
                draws: 1.0,
                turns_per_game: 11.9,
                mean_entropy: Some(0.84),
                ms_per_game: 5200.0,
            },
            StatRow {
                games: 50.0,
                wins: 25.0,
                draws: 2.0,
                turns_per_game: 12.6,
                mean_entropy: Some(0.86),
                ms_per_game: 5100.0,
            },
        ];
        assert!(evaluate_residuals(&honest, &peers).is_none());

        // cheater faking one-hot policy (entropy ~0.05) -> flagged immediately after 3 units
        let fake_policy = vec![
            StatRow {
                games: 50.0,
                wins: 25.0,
                draws: 2.0,
                turns_per_game: 12.0,
                mean_entropy: Some(0.05),
                ms_per_game: 5000.0,
            },
            StatRow {
                games: 50.0,
                wins: 25.0,
                draws: 2.0,
                turns_per_game: 12.2,
                mean_entropy: Some(0.04),
                ms_per_game: 5000.0,
            },
            StatRow {
                games: 50.0,
                wins: 24.0,
                draws: 2.0,
                turns_per_game: 11.8,
                mean_entropy: Some(0.06),
                ms_per_game: 5000.0,
            },
        ];
        let flag = evaluate_residuals(&fake_policy, &peers).expect("fake policy must flag");
        assert!(flag.detail.contains("policy_entropy_z="), "{}", flag.detail);

        // honest slow worker (10x slower) -> NOT flagged
        let slow = vec![
            StatRow {
                games: 50.0,
                wins: 25.0,
                draws: 2.0,
                turns_per_game: 12.0,
                mean_entropy: Some(0.86),
                ms_per_game: 50_000.0,
            },
            StatRow {
                games: 50.0,
                wins: 25.0,
                draws: 2.0,
                turns_per_game: 12.2,
                mean_entropy: Some(0.85),
                ms_per_game: 52_000.0,
            },
            StatRow {
                games: 50.0,
                wins: 24.0,
                draws: 2.0,
                turns_per_game: 11.9,
                mean_entropy: Some(0.87),
                ms_per_game: 49_000.0,
            },
        ];
        assert!(evaluate_residuals(&slow, &peers).is_none());
    }
}
