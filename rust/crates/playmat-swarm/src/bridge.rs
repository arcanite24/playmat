//! S6 Playmat cosmetics bridge: converts matured Mana@Home credits (7 days after acceptance) into Playmat
//! cosmetic capsule tokens atomically and without double-spending (`docs/swarm/S6.md`).
//! Lifetime leaderboard credits remain intact; `credit_redemptions` tracks spent balance.

use crate::store::now;
use crate::{bad, caller, hash, require_not_banned, ApiError, Res, Shared, Swarm};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use playmat_swarm_proto::api::{BridgeRedeemReply, BridgeRedeemRequest, CreditBalance, Role, TrustLevel};
use sqlx::Row;

const DEFAULT_CREDIT_PER_TOKEN: f64 = 100.0;

pub(crate) fn credit_per_token() -> f64 {
    std::env::var("MANA_CREDIT_PER_TOKEN")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.0)
        .unwrap_or(DEFAULT_CREDIT_PER_TOKEN)
}

/// Promotes any pending credits whose 7-day maturation timestamp has passed to `matured`.
pub(crate) async fn promote_matured(s: &Swarm) -> Result<(), sqlx::Error> {
    let t = now() as i64;
    sqlx::query("UPDATE credits SET status = 'matured' WHERE status = 'pending' AND mature_at <= ?")
        .bind(t)
        .execute(&s.db)
        .await?;
    Ok(())
}

async fn token_balances(s: &Swarm, token_id: i64) -> Result<(f64, f64, f64), sqlx::Error> {
    let row = sqlx::query(
        "SELECT \
           COALESCE(SUM(CASE WHEN status = 'pending' THEN credit ELSE 0.0 END), 0.0) AS pending, \
           COALESCE(SUM(CASE WHEN status = 'matured' THEN credit ELSE 0.0 END), 0.0) AS matured \
         FROM credits WHERE token = ?",
    )
    .bind(token_id)
    .fetch_one(&s.db)
    .await?;
    let redeemed: f64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(credit_spent), 0.0) FROM credit_redemptions WHERE token = ?")
            .bind(token_id)
            .fetch_one(&s.db)
            .await?;
    let pending: f64 = row.get("pending");
    let matured_gross: f64 = row.get("matured");
    let matured_avail = (matured_gross - redeemed).max(0.0);
    Ok((pending.max(0.0), matured_avail, redeemed.max(0.0)))
}

/// `GET /api/v1/credits/balance` (authenticated with contributor's worker token).
pub(crate) async fn balance(State(s): State<Shared>, headers: HeaderMap) -> Res<Json<CreditBalance>> {
    let c = caller(&s, &headers, &[Role::Worker, Role::Trainer, Role::Admin]).await?;
    require_not_banned(&c)?;
    promote_matured(&s).await?;
    let (credit_pending, credit_matured, credit_redeemed) = token_balances(&s, c.token).await?;
    let rate = credit_per_token();
    let redeemable_tokens = (credit_matured / rate).floor().max(0.0) as u32;
    Ok(Json(CreditBalance {
        account: c.name,
        credit_pending,
        credit_matured,
        credit_redeemed,
        credit_per_token: rate,
        redeemable_tokens,
    }))
}

/// `POST /api/v1/bridge/redeem` (authenticated with shared `X-Swarm-Bridge-Secret` from Playmat server).
pub(crate) async fn redeem(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(req): Json<BridgeRedeemRequest>,
) -> Res<Json<BridgeRedeemReply>> {
    let expected = std::env::var("SWARM_BRIDGE_SECRET").unwrap_or_default();
    if expected.trim().is_empty() {
        return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, "swarm bridge secret is not configured".into()));
    }
    let provided = headers.get("x-swarm-bridge-secret").and_then(|v| v.to_str().ok()).unwrap_or("");
    if provided != expected {
        return Err(ApiError(StatusCode::UNAUTHORIZED, "invalid swarm bridge secret".into()));
    }
    let playmat_user = req.playmat_user.trim();
    if playmat_user.is_empty() {
        return Err(bad("playmat_user is required"));
    }

    let row = sqlx::query("SELECT id, name, trust, revoked FROM tokens WHERE hash = ?")
        .bind(hash(req.worker_token.trim()))
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "unknown Mana@Home worker token".into()))?;
    if row.get::<i64, _>("revoked") != 0 {
        return Err(ApiError(StatusCode::FORBIDDEN, "worker token has been revoked".into()));
    }
    let trust = TrustLevel::parse(row.get::<String, _>("trust").as_str()).unwrap_or(TrustLevel::Probation);
    if trust == TrustLevel::Banned {
        return Err(ApiError(StatusCode::FORBIDDEN, "account is banned from redeeming credits".into()));
    }
    let token_id: i64 = row.get("id");
    let account: String = row.get("name");

    // Serialize redemptions with the coordinator's lease mutex so concurrent requests cannot double-spend
    let _guard = s.lease.lock().await;
    promote_matured(&s).await?;
    let (_pending, matured_avail, _redeemed) = token_balances(&s, token_id).await?;
    let rate = credit_per_token();
    let capsule_tokens = (matured_avail / rate).floor().max(0.0) as u32;
    if capsule_tokens == 0 {
        return Ok(Json(BridgeRedeemReply {
            account,
            capsule_tokens: 0,
            credit_spent: 0.0,
            credit_remaining: matured_avail,
            redemption_id: None,
        }));
    }

    let credit_spent = capsule_tokens as f64 * rate;
    let credit_remaining = (matured_avail - credit_spent).max(0.0);
    let t = now() as i64;
    let rid: i64 = sqlx::query(
        "INSERT INTO credit_redemptions (token, playmat_user, credit_spent, capsule_tokens, created) \
         VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(token_id)
    .bind(playmat_user)
    .bind(credit_spent)
    .bind(capsule_tokens as i64)
    .bind(t)
    .fetch_one(&s.db)
    .await?
    .get("id");

    s.audit(
        "bridge",
        "redeem",
        &format!("{account} redeemed {credit_spent:.1} credits -> {capsule_tokens} capsule token(s) (#{rid})"),
    )
    .await;
    Ok(Json(BridgeRedeemReply { account, capsule_tokens, credit_spent, credit_remaining, redemption_id: Some(rid) }))
}
