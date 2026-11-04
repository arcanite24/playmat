//! Support's records (Java's `Support` and `SupportStore`, db/support.db): the signed-in account's support code,
//! perks and thank-you, the supporters wall choice, and staff's ledger, links, revokes and grants. Account names are
//! checked against the accounts (db/authorized_user.db, SQLite since Nov 2026) as `AuthorizedUserRepository.getByName`
//! does, case aside.
//!
//! Left to Java: `supportStatus` and `/public/support.json` (Java caches the status for 15 seconds), the looks (they
//! need the Career's catalogs), and the providers' hooks (Discord, the email lookup). A wall change made here reaches
//! Java's cached status within those 15 seconds.

use crate::admin::{
    audit, invalid, java_trim, long, now_millis, object, opt_bool, opt_string, string, AdminStores, Answer, Role, Stop,
};
use playmat_protocol::compare;
use playmat_protocol::handlers::Caller;
use playmat_protocol::jsonrpc::Request;
use serde_json::{json, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection, SqliteRow};
use sqlx::{ConnectOptions, Row};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::time::Duration;

pub const SUPPORT_METHODS: [&str; 8] = [
    "supportPerks",
    "supportNoticeSeen",
    "supportSetWall",
    "adminSupportLedger",
    "adminSupportLink",
    "adminSupportRevoke",
    "adminSupportGrant",
    "adminContributorGrant",
];

const DAY: i64 = 24 * 60 * 60 * 1000;
const SUPPORTER_MILLIS: i64 = 30 * DAY;
const MEMBERSHIP_MILLIS: i64 = 35 * DAY;
const CODE_PREFIX: &str = "PM-";
const CODE_ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
const DEFAULT_GOAL_CENTS: i64 = 4000;
const CONTRIBUTOR_PET: &str = "debug-duck";
/// `SupportPerks.THANK_YOU`, in order
const THANK_YOU: [(&str, &str); 6] = [
    ("sleeve", "supporter"),
    ("playmat", "patron"),
    ("frame", "supporter-ring"),
    ("title", "patron"),
    ("flair", "heart"),
    ("pet", "party-corgi"),
];

const PAYMENT_COLUMNS: &str = "id, provider, provider_id, kind, amount_cents, currency, tier, from_name, message, \
     is_public, account, linked_by, received_at, revoked, revoked_reason, sponsor_id";

/// The accounts, read only, or None when the server keeps them in H2 (the file is missing then).
pub(crate) async fn open_read_only(path: &Path) -> Option<SqliteConnection> {
    if !path.is_file() {
        return None;
    }
    SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .busy_timeout(Duration::from_secs(10))
        .connect()
        .await
        .ok()
}

impl AdminStores {
    pub(crate) async fn support(
        &self,
        caller: &Caller,
        request: &Request,
        admin: &mut SqliteConnection,
        db: &mut SqliteConnection,
        mut accounts: Option<&mut SqliteConnection>,
        audits: &mut Vec<String>,
    ) -> Answer {
        let p = &request.params;
        let now = now_millis();
        match request.method.as_str() {
            "supportPerks" => {
                let account = signed_in(caller, accounts.as_deref_mut()).await?;
                you(db, &account, now).await
            }
            "supportNoticeSeen" => {
                let account = signed_in(caller, accounts.as_deref_mut()).await?;
                let cleared =
                    sqlx::query("DELETE FROM notices WHERE account = ?").bind(&account).execute(&mut *db).await?;
                Ok(json!(cleared.rows_affected() > 0))
            }
            "supportSetWall" => {
                let account = signed_in(caller, accounts.as_deref_mut()).await?;
                let Some(Value::Bool(show)) = p.first() else { return Err(Stop::Decline) };
                sqlx::query("INSERT OR REPLACE INTO prefs (account, wall) VALUES (?, ?)")
                    .bind(&account)
                    .bind(i64::from(*show))
                    .execute(&mut *db)
                    .await?;
                you(db, &account, now).await
            }
            "adminSupportLedger" => {
                self.require(caller, admin, Role::Admin).await?;
                let month = opt_string(p, 0)?.unwrap_or_default();
                let unmatched_only = opt_bool(p, 1, false)?;
                let provider = opt_string(p, 2)?.unwrap_or_default();
                ledger(admin, db, &month, unmatched_only, &provider, now).await
            }
            "adminSupportLink" => {
                let actor = self.require(caller, admin, Role::Admin).await?;
                let id = long(p, 0)?;
                let user_name = string(p, 1)?;
                let Some(found) = payment(db, id).await? else {
                    return Err(invalid(format!("There is no payment {id}")));
                };
                if found.get("revoked").and_then(Value::as_bool) == Some(true) {
                    return Err(invalid(format!("Payment {id} was revoked")));
                }
                let Some(account) = account_name(accounts.as_deref_mut(), java_trim(&user_name)).await? else {
                    return Err(invalid(format!("There is no account named {user_name}")));
                };
                sqlx::query("UPDATE payments SET account = ?, linked_by = ? WHERE id = ?")
                    .bind(&account)
                    .bind("staff")
                    .bind(id)
                    .execute(&mut *db)
                    .await?;
                granted(db, &account, id, now, true).await?;
                let payment = payment(db, id).await?.ok_or(Stop::Decline)?;
                let target = payment.get("account").and_then(Value::as_str).map(str::to_string);
                audit(
                    admin,
                    audits,
                    &actor,
                    "support.link",
                    target.as_deref(),
                    Some(&format!("payment {id}")),
                    None,
                    now,
                )
                .await?;
                Ok(payment)
            }
            "adminSupportRevoke" => {
                let actor = self.require(caller, admin, Role::Admin).await?;
                let id = long(p, 0)?;
                let reason = java_trim(&string(p, 1)?).to_string();
                if reason.is_empty() {
                    return Err(invalid("Say why the payment is revoked; it goes in the audit log"));
                }
                let remove_perks = opt_bool(p, 2, false)?;
                let Some(before) = payment(db, id).await? else {
                    return Err(invalid(format!("There is no payment {id}")));
                };
                sqlx::query("UPDATE payments SET revoked = 1, revoked_reason = ? WHERE id = ?")
                    .bind(&reason)
                    .bind(id)
                    .execute(&mut *db)
                    .await?;
                if let (true, Some(account)) = (remove_perks, before.get("account").and_then(Value::as_str)) {
                    let removed =
                        sqlx::query("DELETE FROM perks WHERE payment_id = ?").bind(id).execute(&mut *db).await?;
                    if removed.rows_affected() > 0 {
                        // another payment of the account still stands for them
                        for other in payments(db, "WHERE account = ? ORDER BY received_at, id", &[account]).await? {
                            let other_id = other.get("id").and_then(Value::as_i64).unwrap_or(0);
                            if other.get("revoked").and_then(Value::as_bool) != Some(true) && other_id != id {
                                granted(db, account, other_id, now, false).await?;
                                break;
                            }
                        }
                    }
                }
                let payment = payment(db, id).await?.ok_or(Stop::Decline)?;
                let target = payment.get("account").and_then(Value::as_str).map(str::to_string);
                let detail = format!("payment {id}{}", if remove_perks { ", perks removed" } else { "" });
                audit(admin, audits, &actor, "support.revoke", target.as_deref(), Some(&detail), Some(&reason), now)
                    .await?;
                Ok(payment)
            }
            "adminSupportGrant" | "adminContributorGrant" => {
                let actor = self.require(caller, admin, Role::Admin).await?;
                let reason = java_trim(&string(p, 1)?).to_string();
                if reason.is_empty() {
                    return Err(invalid("Say why; it goes in the audit log"));
                }
                let name = java_trim(&string(p, 0)?).to_string();
                if name.is_empty() {
                    return Err(invalid("Which account?"));
                }
                let Some(account) = account_name(accounts, &name).await? else {
                    return Err(invalid(format!("There is no account named {name}")));
                };
                if request.method == "adminSupportGrant" {
                    granted(db, &account, 0, now, true).await?;
                    // Java reads the account back by the name staff typed
                    let you = you(db, &name, now).await?;
                    audit(admin, audits, &actor, "support.grant", Some(&name), None, Some(&reason), now).await?;
                    Ok(you)
                } else {
                    let added = add_perk(db, &account, "pet", CONTRIBUTOR_PET, now, 0).await?;
                    let detail = if added { None } else { Some("already had it") };
                    audit(admin, audits, &actor, "support.contributor", Some(&name), detail, Some(&reason), now)
                        .await?;
                    Ok(json!(added))
                }
            }
            _ => Err(Stop::Decline),
        }
    }
}

/// `MyAccountApi.account`: the signed-in account's name as stored. The admin console and servers without accounts
/// are Java's to answer.
async fn signed_in(caller: &Caller, accounts: Option<&mut SqliteConnection>) -> Result<String, Stop> {
    let (true, false, Some(user)) = (caller.account, caller.admin, &caller.user) else { return Err(Stop::Decline) };
    match account_name(accounts, user).await? {
        Some(name) => Ok(name),
        None => Err(invalid(format!("There is no account named {user}"))),
    }
}

/// `AuthorizedUserRepository.getByName`: the account's name as stored, in any case. Without the SQLite accounts (a
/// server on H2, or without accounts) Java answers.
async fn account_name(accounts: Option<&mut SqliteConnection>, name: &str) -> Result<Option<String>, Stop> {
    let Some(accounts) = accounts else { return Err(Stop::Decline) };
    // H2's IGNORECASE folds any letter; SQLite's NOCASE only ASCII ones, so other names are left to Java
    if !name.is_ascii() {
        return Err(Stop::Decline);
    }
    Ok(sqlx::query_scalar("SELECT name FROM authorized_user WHERE name = ?")
        .bind(name)
        .fetch_optional(&mut *accounts)
        .await?)
}

/// `Support.you`: the code (made on the first ask), perks, supporter status, wall and thank-you
async fn you(db: &mut SqliteConnection, account: &str, now: i64) -> Answer {
    let code = code_for(db, account, now).await?;
    let mut perks = Vec::new();
    let mut since = 0;
    let rows = sqlx::query(
        "SELECT kind, item, granted_at, payment_id FROM perks WHERE account = ? ORDER BY granted_at, kind, item",
    )
    .bind(account)
    .fetch_all(&mut *db)
    .await?;
    for row in rows {
        let granted_at: i64 = row.try_get("granted_at")?;
        perks.push(json!({"kind": row.try_get::<String, _>("kind")?, "item": row.try_get::<String, _>("item")?, "grantedAt": granted_at}));
        since = if since == 0 { granted_at } else { since.min(granted_at) };
    }
    let mut supporter = false;
    for payment in payments(db, "WHERE account = ? ORDER BY received_at, id", &[account]).await? {
        if payment["revoked"] == json!(true) {
            continue;
        }
        let received = payment["receivedAt"].as_i64().unwrap_or(0);
        since = if since == 0 { received } else { since.min(received) };
        let age = now - received;
        if age < SUPPORTER_MILLIS || (payment["kind"] == json!("membership") && age < MEMBERSHIP_MILLIS) {
            supporter = true;
        }
    }
    let sponsorships = sqlx::query("SELECT active, until FROM sponsorships WHERE account = ? ORDER BY since")
        .bind(account)
        .fetch_all(&mut *db)
        .await?;
    for row in sponsorships {
        supporter |= runs(row.try_get("active")?, row.try_get("until")?, now);
    }
    let wall: Option<i64> =
        sqlx::query_scalar("SELECT wall FROM prefs WHERE account = ?").bind(account).fetch_optional(&mut *db).await?;
    let notice: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM notices WHERE account = ?").bind(account).fetch_optional(&mut *db).await?;
    Ok(object([
        ("code", code.map_or(Value::Null, Value::from)),
        ("supporter", json!(supporter)),
        ("since", json!(since)),
        ("perks", Value::Array(perks)),
        ("wall", json!(wall.unwrap_or(0) != 0)),
        ("notice", json!(notice.is_some())),
    ]))
}

/// `Sponsorship.runs`
fn runs(active: i64, until: i64, now: i64) -> bool {
    active != 0 && (until == 0 || until > now)
}

/// `Support.codeFor`: PM- and four letters, longer after collisions
async fn code_for(db: &mut SqliteConnection, account: &str, now: i64) -> Result<Option<String>, Stop> {
    let mut code: Option<String> =
        sqlx::query_scalar("SELECT code FROM codes WHERE account = ?").bind(account).fetch_optional(&mut *db).await?;
    let mut tries = 0;
    while code.is_none() && tries < 60 {
        let length = 4 + tries / 20;
        let mut bytes = vec![0u8; length];
        getrandom::getrandom(&mut bytes).map_err(|_| Stop::Decline)?;
        let candidate: String = CODE_PREFIX
            .chars()
            .chain(bytes.iter().map(|b| CODE_ALPHABET[usize::from(*b) % CODE_ALPHABET.len()] as char))
            .collect();
        let added = sqlx::query("INSERT OR IGNORE INTO codes (account, code, created_at) VALUES (?, ?, ?)")
            .bind(account)
            .bind(&candidate)
            .bind(now)
            .execute(&mut *db)
            .await?;
        if added.rows_affected() == 1 {
            code = Some(candidate);
        }
        tries += 1;
    }
    Ok(code)
}

async fn add_perk(
    db: &mut SqliteConnection,
    account: &str,
    kind: &str,
    item: &str,
    now: i64,
    payment_id: i64,
) -> Result<bool, Stop> {
    let added =
        sqlx::query("INSERT OR IGNORE INTO perks (account, kind, item, granted_at, payment_id) VALUES (?, ?, ?, ?, ?)")
            .bind(account)
            .bind(kind)
            .bind(item)
            .bind(now)
            .bind(payment_id)
            .execute(&mut *db)
            .await?;
    Ok(added.rows_affected() == 1)
}

/// `Support.granted`: the thank-you set, and a thank-you notice when anything was new
async fn granted(
    db: &mut SqliteConnection,
    account: &str,
    payment_id: i64,
    now: i64,
    notify: bool,
) -> Result<(), Stop> {
    let mut any = false;
    for (kind, item) in THANK_YOU {
        any |= add_perk(db, account, kind, item, now, payment_id).await?;
    }
    if any && notify {
        sqlx::query("INSERT OR REPLACE INTO notices (account, payment_id, at) VALUES (?, ?, ?)")
            .bind(account)
            .bind(payment_id)
            .bind(now)
            .execute(&mut *db)
            .await?;
    }
    Ok(())
}

async fn payment(db: &mut SqliteConnection, id: i64) -> Result<Option<Value>, Stop> {
    let id = id.to_string();
    Ok(payments(db, "WHERE id = ?", &[&id]).await?.into_iter().next())
}

async fn payments(db: &mut SqliteConnection, filter: &str, binds: &[&str]) -> Result<Vec<Value>, Stop> {
    let sql = format!("SELECT {PAYMENT_COLUMNS} FROM payments {filter}");
    let mut query = sqlx::query(&sql);
    for bind in binds {
        query = query.bind(*bind);
    }
    let rows = query.fetch_all(&mut *db).await?;
    rows.iter().map(payment_json).collect()
}

/// `Payment` as Gson writes it: declaration order, nulls left out, the email hash and payload never
fn payment_json(row: &SqliteRow) -> Result<Value, Stop> {
    let text = |column: &str| -> Result<Value, Stop> {
        Ok(row.try_get::<Option<String>, _>(column)?.map_or(Value::Null, Value::from))
    };
    let number =
        |column: &str| -> Result<Value, Stop> { Ok(json!(row.try_get::<Option<i64>, _>(column)?.unwrap_or(0))) };
    let flag =
        |column: &str| -> Result<Value, Stop> { Ok(json!(row.try_get::<Option<i64>, _>(column)?.unwrap_or(0) != 0)) };
    Ok(object([
        ("id", number("id")?),
        ("provider", text("provider")?),
        ("providerId", text("provider_id")?),
        ("kind", text("kind")?),
        ("amountCents", number("amount_cents")?),
        ("currency", text("currency")?),
        ("tier", text("tier")?),
        ("fromName", text("from_name")?),
        ("message", text("message")?),
        ("isPublic", flag("is_public")?),
        ("account", text("account")?),
        ("linkedBy", text("linked_by")?),
        ("receivedAt", number("received_at")?),
        ("revoked", flag("revoked")?),
        ("revokedReason", text("revoked_reason")?),
        ("sponsorId", number("sponsor_id")?),
    ]))
}

/// a calendar month, UTC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Month {
    year: i64,
    month: i64,
}

impl Month {
    fn of(millis: i64) -> Month {
        let (year, month, _) = civil(millis.div_euclid(DAY));
        Month { year, month }
    }

    /// `YearMonth.parse` for four-digit years; anything else Java reads is left to it
    fn parse(text: &str) -> Result<Month, Stop> {
        let bytes = text.as_bytes();
        if bytes.len() == 7 && bytes[4] == b'-' && text.chars().enumerate().all(|(i, c)| i == 4 || c.is_ascii_digit()) {
            let year = text[..4].parse().map_err(|_| Stop::Decline)?;
            let month: i64 = text[5..].parse().map_err(|_| Stop::Decline)?;
            if (1..=12).contains(&month) {
                return Ok(Month { year, month });
            }
        }
        if text.starts_with(['+', '-']) {
            return Err(Stop::Decline);
        }
        Err(invalid("The month is written like 2026-10"))
    }

    fn next(self) -> Month {
        if self.month == 12 {
            Month { year: self.year + 1, month: 1 }
        } else {
            Month { year: self.year, month: self.month + 1 }
        }
    }

    fn start(self) -> i64 {
        days_from_civil(self.year, self.month, 1) * DAY
    }

    fn label(self) -> String {
        format!("{:04}-{:02}", self.year, self.month)
    }
}

/// days since 1970-01-01 to (year, month, day), proleptic Gregorian
pub(crate) fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

pub(crate) fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `Support.ensurePeriods`: each running sponsorship's payment for this month; ended ones stop
async fn ensure_periods(db: &mut SqliteConnection, now: i64) -> Result<(), Stop> {
    let rows = sqlx::query(
        "SELECT github_id, login, node_id, monthly_cents, tier, is_public, active, since, until, account \
         FROM sponsorships ORDER BY since",
    )
    .fetch_all(&mut *db)
    .await?;
    for row in rows {
        let github_id: i64 = row.try_get("github_id")?;
        let active: i64 = row.try_get("active")?;
        let until: i64 = row.try_get("until")?;
        if active != 0 && until > 0 && until <= now {
            // putSponsorship: the row again, inactive
            sqlx::query("UPDATE sponsorships SET active = 0, updated_at = ? WHERE github_id = ?")
                .bind(now)
                .bind(github_id)
                .execute(&mut *db)
                .await?;
            continue;
        }
        let monthly: i64 = row.try_get("monthly_cents")?;
        let node_id: Option<String> = row.try_get("node_id")?;
        let Some(node_id) = node_id else { continue };
        if !runs(active, until, now) || monthly <= 0 {
            continue;
        }
        let month = Month::of(now);
        let account: Option<String> = row.try_get("account")?;
        let since: i64 = row.try_get("since")?;
        let added = sqlx::query(
            "INSERT OR IGNORE INTO payments (provider, provider_id, kind, amount_cents, currency, tier, from_name, \
             message, is_public, email_hash, account, linked_by, received_at, raw_payload, revoked, revoked_reason, \
             sponsor_id) VALUES ('github', ?, 'sponsorship', ?, 'USD', ?, ?, NULL, ?, NULL, ?, ?, ?, NULL, 0, NULL, ?)",
        )
        .bind(format!("{node_id}:{}", month.label()))
        .bind(monthly)
        .bind(row.try_get::<Option<String>, _>("tier")?)
        .bind(row.try_get::<Option<String>, _>("login")?)
        .bind(row.try_get::<i64, _>("is_public")?)
        .bind(account.as_deref())
        .bind(account.as_ref().map(|_| "auto"))
        .bind(since.max(month.start()))
        .bind(github_id)
        .execute(&mut *db)
        .await?;
        if added.rows_affected() == 1 {
            if let Some(account) = &account {
                granted(db, account, added.last_insert_rowid(), now, true).await?;
            }
        }
    }
    Ok(())
}

/// `Support.ledger`
async fn ledger(
    admin: &mut SqliteConnection,
    db: &mut SqliteConnection,
    month: &str,
    unmatched_only: bool,
    provider: &str,
    now: i64,
) -> Answer {
    ensure_periods(db, now).await?;
    let month = if java_trim(month).is_empty() { Month::of(now) } else { Month::parse(java_trim(month))? };
    let (from, to) = (month.start().to_string(), month.next().start().to_string());
    let month_payments =
        payments(db, "WHERE received_at >= ? AND received_at < ? ORDER BY received_at DESC, id DESC", &[&from, &to])
            .await?;
    let goal: Option<Option<String>> = sqlx::query_scalar("SELECT `value` FROM `admin_setting` WHERE `name` = ?")
        .bind("runtime.support.goalCents")
        .fetch_optional(&mut *admin)
        .await?;
    let goal_cents = match goal.flatten().filter(|v| !v.is_empty()) {
        Some(text) => text.parse::<i64>().map_err(|_| Stop::Decline)?,
        None => DEFAULT_GOAL_CENTS,
    };
    let live = |p: &&Value| p["revoked"] != json!(true);
    let month_cents: i64 = month_payments
        .iter()
        .filter(live)
        .filter(|p| p["currency"] == json!("USD"))
        .filter_map(|p| p["amountCents"].as_i64())
        .sum();
    let mut other: BTreeMap<String, i64> = BTreeMap::new();
    for payment in month_payments.iter().filter(live) {
        if let Some(currency) = payment["currency"].as_str().filter(|c| *c != "USD") {
            *other.entry(currency.to_string()).or_default() += payment["amountCents"].as_i64().unwrap_or(0);
        }
    }
    let unmatched: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM payments WHERE account IS NULL AND revoked = 0")
        .fetch_one(&mut *db)
        .await?;
    let shown = if unmatched_only {
        payments(db, "WHERE account IS NULL AND revoked = 0 ORDER BY received_at DESC, id DESC LIMIT 500", &[]).await?
    } else {
        month_payments.clone()
    };
    let provider = java_trim(provider);
    let shown: Vec<Value> = shown
        .into_iter()
        .filter(|p| {
            provider.is_empty() || p["provider"].as_str().is_some_and(|name| name.eq_ignore_ascii_case(provider))
        })
        .collect();
    Ok(object([
        ("month", json!(month.label())),
        ("payments", Value::Array(shown)),
        ("monthCents", json!(month_cents)),
        ("goalCents", json!(goal_cents)),
        ("supporters", json!(supporters(db, &from, &to).await?)),
        (
            "otherCurrencies",
            Value::Array(other.iter().map(|(c, cents)| json!(format!("{c} {}", amount(*cents)))).collect()),
        ),
        ("unmatched", json!(unmatched)),
    ]))
}

/// `Support.supporters`: payers, counted once each (by account, sponsor, email hash, or payment)
async fn supporters(db: &mut SqliteConnection, from: &str, to: &str) -> Result<usize, Stop> {
    let rows = sqlx::query(
        "SELECT id, account, sponsor_id, email_hash FROM payments WHERE received_at >= ? AND received_at < ? AND revoked = 0",
    )
    .bind(from)
    .bind(to)
    .fetch_all(&mut *db)
    .await?;
    let mut payers = HashSet::new();
    for row in rows {
        let account: Option<String> = row.try_get("account")?;
        let sponsor: i64 = row.try_get::<Option<i64>, _>("sponsor_id")?.unwrap_or(0);
        let email: Option<String> = row.try_get("email_hash")?;
        payers.insert(match (account, email) {
            (Some(account), _) => format!("a:{}", account.to_lowercase()),
            _ if sponsor > 0 => format!("g:{sponsor}"),
            (None, Some(email)) => format!("e:{email}"),
            (None, None) => format!("p:{}", row.try_get::<i64, _>("id")?),
        });
    }
    Ok(payers.len())
}

/// `Support.amount`: dollars and cents
fn amount(cents: i64) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

/// Shadow comparison: grants made a moment apart differ in their times, and a code made on the first ask is random.
pub fn difference(method: &str, java: &Value, rust: &Value) -> Option<String> {
    let mut rust = rust.clone();
    if let (Value::Object(j), Value::Object(r)) = (java, &mut rust) {
        let code = |v: Option<&Value>| v.and_then(Value::as_str).is_some_and(|c| c.starts_with(CODE_PREFIX));
        if code(j.get("code")) && code(r.get("code")) {
            r.insert("code".into(), j["code"].clone());
        }
        let close = |a: Option<&Value>, b: Option<&Value>| match (a.and_then(Value::as_i64), b.and_then(Value::as_i64))
        {
            (Some(a), Some(b)) => (a - b).abs() <= 60_000,
            _ => false,
        };
        if close(j.get("since"), r.get("since")) {
            r.insert("since".into(), j["since"].clone());
        }
        if let (Some(Value::Array(jp)), Some(Value::Array(rp))) = (j.get("perks"), r.get_mut("perks")) {
            for (a, b) in jp.iter().zip(rp.iter_mut()) {
                if close(a.get("grantedAt"), b.get("grantedAt")) {
                    b["grantedAt"] = a["grantedAt"].clone();
                }
            }
        }
    }
    let _ = method;
    compare::difference(java, &rust)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn months() {
        let november = Month::parse("2026-11").ok().unwrap();
        assert_eq!(november.start(), 1_793_491_200_000);
        assert_eq!(Month::of(1_793_491_200_000), november);
        assert_eq!(Month::of(1_793_491_199_999).label(), "2026-10");
        assert_eq!(Month::parse("2026-12").ok().unwrap().next().label(), "2027-01");
        assert!(matches!(Month::parse("2026-13"), Err(Stop::Error(..))));
        assert!(matches!(Month::parse("Nov"), Err(Stop::Error(..))));
        assert!(matches!(Month::parse("+2026-11"), Err(Stop::Decline)));
        assert_eq!(amount(1234), "12.34");
        assert_eq!(amount(5), "0.05");
    }

    use playmat_protocol::jsonrpc::Outcome;

    fn caller(user: &str) -> Caller {
        Caller { user: Some(user.into()), admin: false, account: true, ip: "127.0.0.1".into(), connection: 1 }
    }

    fn request(method: &str, params: Value) -> Request {
        Request { id: Some(json!(1)), method: method.into(), params: params.as_array().cloned().unwrap_or_default() }
    }

    fn result(outcome: Outcome) -> Value {
        match outcome {
            Outcome::Result(value) => value,
            other => panic!("expected a result, got {other:?}"),
        }
    }

    fn error(outcome: Outcome) -> String {
        match outcome {
            Outcome::Error { message, .. } => message,
            other => panic!("expected an error, got {other:?}"),
        }
    }

    /// Java's SupportStore migrations, the accounts as the server makes them, and a staff admin
    async fn stores() -> (AdminStores, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("playmat-support-{}", uuid::Uuid::new_v4()));
        let mut stores = AdminStores::open(&dir.join("web_admin.db"), None, "Boss").await.unwrap();
        let open = async |name: &str| {
            SqliteConnectOptions::new().filename(dir.join(name)).create_if_missing(true).connect().await
        };
        let mut support = open("support.db").await.unwrap();
        for sql in [
            "CREATE TABLE payments (id INTEGER PRIMARY KEY AUTOINCREMENT, provider TEXT NOT NULL, provider_id TEXT NOT NULL UNIQUE, \
             kind TEXT NOT NULL, amount_cents INTEGER NOT NULL, currency TEXT NOT NULL, tier TEXT, from_name TEXT, message TEXT, \
             is_public INTEGER NOT NULL DEFAULT 0, email_hash TEXT, account TEXT COLLATE NOCASE, linked_by TEXT, \
             received_at INTEGER NOT NULL, raw_payload TEXT, revoked INTEGER NOT NULL DEFAULT 0, revoked_reason TEXT, \
             sponsor_id INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE codes (account TEXT PRIMARY KEY COLLATE NOCASE, code TEXT NOT NULL UNIQUE, created_at INTEGER NOT NULL)",
            "CREATE TABLE perks (account TEXT NOT NULL COLLATE NOCASE, kind TEXT NOT NULL, item TEXT NOT NULL, \
             granted_at INTEGER NOT NULL, payment_id INTEGER, PRIMARY KEY (account, kind, item))",
            "CREATE TABLE notices (account TEXT PRIMARY KEY COLLATE NOCASE, payment_id INTEGER, at INTEGER NOT NULL)",
            "CREATE TABLE sponsorships (github_id INTEGER PRIMARY KEY, login TEXT, node_id TEXT, monthly_cents INTEGER NOT NULL, \
             tier TEXT, is_public INTEGER NOT NULL DEFAULT 0, active INTEGER NOT NULL, since INTEGER NOT NULL, \
             until INTEGER NOT NULL DEFAULT 0, account TEXT COLLATE NOCASE, updated_at INTEGER NOT NULL)",
            "CREATE TABLE prefs (account TEXT PRIMARY KEY COLLATE NOCASE, wall INTEGER NOT NULL DEFAULT 0)",
            "INSERT INTO payments (provider, provider_id, kind, amount_cents, currency, from_name, is_public, email_hash, \
             received_at) VALUES ('kofi', 'k1', 'donation', 500, 'USD', 'Kind Stranger', 1, 'h1', 1789000000000), \
             ('kofi', 'k2', 'donation', 300, 'EUR', NULL, 0, 'h1', 1789000001000)",
            "INSERT INTO sponsorships (github_id, login, node_id, monthly_cents, tier, is_public, active, since, account, \
             updated_at) VALUES (7, 'octo', 'S_7', 1000, 'Gold', 1, 1, 1790000000000, 'Alice', 0)",
        ] {
            sqlx::query(sql).execute(&mut support).await.unwrap();
        }
        let mut accounts = open("authorized_user.db").await.unwrap();
        sqlx::query(
            "CREATE TABLE authorized_user (name VARCHAR COLLATE NOCASE PRIMARY KEY, email VARCHAR COLLATE NOCASE)",
        )
        .execute(&mut accounts)
        .await
        .unwrap();
        sqlx::query("INSERT INTO authorized_user (name) VALUES ('Alice'), ('bob'), ('boss')")
            .execute(&mut accounts)
            .await
            .unwrap();
        let mut admin = open("web_admin.db").await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS `admin_setting` (`name` VARCHAR , `value` VARCHAR , `changedBy` VARCHAR , `changedAt` BIGINT , PRIMARY KEY (`name`) )")
            .execute(&mut admin)
            .await
            .unwrap();
        stores.add_support(&dir.join("support.db"), &dir.join("authorized_user.db")).await.unwrap();
        (stores, dir)
    }

    #[tokio::test]
    async fn perks_wall_and_notice() {
        let (stores, dir) = stores().await;
        let call = async |user: &str, method: &str, params: Value| {
            stores.call(&caller(user), &request(method, params), true).await
        };
        // a new account: a code made on the first ask, kept after
        let you = result(call("BOB", "supportPerks", json!([])).await);
        let code = you["code"].as_str().unwrap().to_string();
        assert!(code.starts_with("PM-") && code.len() == 7, "{code}");
        assert_eq!(you["supporter"], json!(false));
        assert_eq!(result(call("bob", "supportPerks", json!([])).await)["code"], json!(code));
        assert_eq!(result(call("bob", "supportSetWall", json!([true])).await)["wall"], json!(true));
        assert!(matches!(call("bob", "supportSetWall", json!(["yes"])).await, Outcome::Declined));
        assert_eq!(result(call("bob", "supportNoticeSeen", json!([])).await), json!(false));
        assert_eq!(error(call("ghost", "supportPerks", json!([])).await), "There is no account named ghost");

        // staff: players can't, the owner can; the thank-you set comes with a notice
        assert_eq!(error(call("bob", "adminSupportGrant", json!(["bob", "kofi by mail"])).await), "Staff only");
        assert_eq!(
            error(call("boss", "adminSupportGrant", json!(["bob", " "])).await),
            "Say why; it goes in the audit log"
        );
        assert_eq!(
            error(call("boss", "adminSupportGrant", json!(["nobody", "x"])).await),
            "There is no account named nobody"
        );
        let granted = result(call("boss", "adminSupportGrant", json!([" bob ", "kofi by mail"])).await);
        assert_eq!(granted["perks"].as_array().unwrap().len(), 6);
        assert_eq!(granted["notice"], json!(true));
        assert_eq!(result(call("bob", "supportNoticeSeen", json!([])).await), json!(true));
        assert_eq!(result(call("boss", "adminContributorGrant", json!(["BOB", "bug reports"])).await), json!(true));
        assert_eq!(result(call("boss", "adminContributorGrant", json!(["bob", "again"])).await), json!(false));
        assert_eq!(result(call("bob", "supportPerks", json!([])).await)["perks"].as_array().unwrap().len(), 7);
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn ledger_links_and_revokes() {
        let (stores, dir) = stores().await;
        let staff = caller("Boss");
        let call = async |method: &str, params: Value| stores.call(&staff, &request(method, params), true).await;
        let now = now_millis();
        // the running sponsorship's month is added on the first look, linked to Alice
        let ledger = result(call("adminSupportLedger", json!([])).await);
        assert_eq!(ledger["month"], json!(Month::of(now).label()));
        assert_eq!(ledger["goalCents"], json!(4000));
        let sponsored = &ledger["payments"].as_array().unwrap()[0];
        assert_eq!(sponsored["providerId"], json!(format!("S_7:{}", Month::of(now).label())));
        assert_eq!(sponsored["account"], json!("Alice"));
        assert_eq!(sponsored["linkedBy"], json!("auto"));
        assert_eq!(result(call("adminSupportLedger", json!([])).await)["payments"].as_array().unwrap().len(), 1);

        let september = result(call("adminSupportLedger", json!(["2026-09", null, " KOFI "])).await);
        assert_eq!(september["payments"].as_array().unwrap().len(), 2);
        assert_eq!(september["monthCents"], json!(500));
        assert_eq!(september["otherCurrencies"], json!(["EUR 3.00"]));
        // the two Ko-fi payments share an email: one supporter
        assert_eq!(september["supporters"], json!(1));
        assert_eq!(september["unmatched"], json!(2));
        assert!(september["payments"][0].get("emailHash").is_none());
        assert_eq!(error(call("adminSupportLedger", json!(["November"])).await), "The month is written like 2026-10");

        let linked = result(call("adminSupportLink", json!([1, "alice"])).await);
        assert_eq!((linked["account"].clone(), linked["linkedBy"].clone()), (json!("Alice"), json!("staff")));
        assert_eq!(error(call("adminSupportLink", json!([99, "alice"])).await), "There is no payment 99");
        let revoked = result(call("adminSupportRevoke", json!([1, "refund", true])).await);
        assert_eq!((revoked["revoked"].clone(), revoked["revokedReason"].clone()), (json!(true), json!("refund")));
        assert_eq!(error(call("adminSupportLink", json!([1, "alice"])).await), "Payment 1 was revoked");
        // the sponsorship's month still stands for Alice's perks
        let alice = result(stores.call(&caller("alice"), &request("supportPerks", json!([])), true).await);
        assert_eq!(alice["perks"].as_array().unwrap().len(), 6);
        assert_eq!(alice["supporter"], json!(true));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn shadow_tolerance() {
        let java =
            json!({"code": "PM-ABCD", "since": 1000, "perks": [{"kind": "pet", "grantedAt": 1000}], "wall": false});
        let rust =
            json!({"code": "PM-WXYZ", "since": 2000, "perks": [{"kind": "pet", "grantedAt": 2000}], "wall": false});
        assert_eq!(difference("supportPerks", &java, &rust), None);
        let wall =
            json!({"code": "PM-WXYZ", "since": 2000, "perks": [{"kind": "pet", "grantedAt": 2000}], "wall": true});
        assert!(difference("supportPerks", &java, &wall).is_some());
    }
}
