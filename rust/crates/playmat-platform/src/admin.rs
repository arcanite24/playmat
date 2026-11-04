//! The staff console's calls that live in the bridge's own databases and nowhere in Java's memory: staff roles, the
//! audit log, player reports, client errors, the allowlist and invites, on the same SQLite files as Java's
//! `AdminStore` (db/web_admin.db) and `ReportStore` (db/web_reports.db).
//!
//! Who may call is decided as `Staff.member` does: the admin password's console session is an owner, accounts on the
//! server (`XMAGE_AUTH`) are staff when `admin_staff` gives them a role (read on every call), and the configured owner
//! (`XMAGE_OWNER`) always is. The calls that need Java's memory (the access mode and notice, runtime settings, live
//! users and tables, mail and Discord, the H2 accounts) stay with Java.
//!
//! Each call runs in one transaction with its audit entry. In `shadow` it is rolled back, because Java makes the change.
//! A database error is rolled back and left to Java.

use playmat_protocol::compare;
use playmat_protocol::handlers::Caller;
use playmat_protocol::jsonrpc::{Outcome, Request, INVALID_PARAMS, NOT_AUTHORIZED};
use serde_json::{json, Map, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection, SqlitePool, SqlitePoolOptions, SqliteRow};
use sqlx::Row;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

/// answered from db/web_admin.db
pub const ADMIN_METHODS: [&str; 11] = [
    "staffMe",
    "adminStaff",
    "adminAudit",
    "adminClientErrors",
    "adminClientErrorResolve",
    "adminAllowAdd",
    "adminInviteCreate",
    "adminInviteRevoke",
    "inviteCheck",
    "adminFeedback",
    "adminFeedbackMark",
];
/// answered from db/web_reports.db (and the audit log in db/web_admin.db)
pub const REPORT_METHODS: [&str; 2] = ["adminGetReports", "adminCloseReport"];

const MAX_NOTE: usize = 300;
const MAX_REASON: usize = 500;
const MAX_INVITE_USES: i64 = 100;
const MAX_INVITE_DAYS: i64 = 90;
const DAY_MILLIS: i64 = 24 * 60 * 60 * 1000;

const ADMIN_TABLES: [&str; 9] = [
    "CREATE TABLE IF NOT EXISTS `audit_entry` (`id` INTEGER PRIMARY KEY AUTOINCREMENT , `at` BIGINT , `actor` VARCHAR , \
     `action` VARCHAR , `target` VARCHAR , `detail` VARCHAR , `reason` VARCHAR )",
    "CREATE INDEX IF NOT EXISTS `audit_entry_at_idx` ON `audit_entry` ( `at` )",
    "CREATE INDEX IF NOT EXISTS `audit_entry_target_idx` ON `audit_entry` ( `target` )",
    "CREATE INDEX IF NOT EXISTS `audit_entry_action_idx` ON `audit_entry` ( `action` )",
    "CREATE INDEX IF NOT EXISTS `audit_entry_actor_idx` ON `audit_entry` ( `actor` )",
    "CREATE TABLE IF NOT EXISTS `allow_entry` (`id` INTEGER PRIMARY KEY AUTOINCREMENT , `name` VARCHAR , `email` VARCHAR , \
     `note` VARCHAR , `addedBy` VARCHAR , `addedAt` BIGINT )",
    "CREATE TABLE IF NOT EXISTS `invite` (`code` VARCHAR , `note` VARCHAR , `createdBy` VARCHAR , `createdAt` BIGINT , \
     `expiresAt` BIGINT , `maxUses` INTEGER , `uses` INTEGER , `revoked` BOOLEAN , `usedBy` VARCHAR , PRIMARY KEY (`code`) )",
    "CREATE TABLE IF NOT EXISTS `admin_staff` (`key` VARCHAR , `name` VARCHAR , `role` VARCHAR , `grantedBy` VARCHAR , \
     `grantedAt` BIGINT , PRIMARY KEY (`key`) )",
    "CREATE TABLE IF NOT EXISTS `admin_client_error` (`id` INTEGER PRIMARY KEY AUTOINCREMENT , `fingerprint` VARCHAR , \
     `message` VARCHAR , `source` VARCHAR , `path` VARCHAR , `stack` VARCHAR , `appVersion` VARCHAR , `userAgent` VARCHAR , \
     `context` VARCHAR , `lastUser` VARCHAR , `count` INTEGER , `firstSeen` BIGINT , `lastSeen` BIGINT , `resolved` BOOLEAN , \
     `resolvedBy` VARCHAR , `resolvedAt` BIGINT , `regressed` BOOLEAN ,  UNIQUE (`fingerprint`))",
];

const REPORT_TABLES: [&str; 1] = [
    "CREATE TABLE IF NOT EXISTS `player_report` (`id` INTEGER PRIMARY KEY AUTOINCREMENT , `reporter` VARCHAR , \
     `reported` VARCHAR , `reason` VARCHAR , `details` VARCHAR , `gameId` VARCHAR , `createdAt` BIGINT , `open` BOOLEAN , \
     `resolution` VARCHAR , `closedAt` BIGINT )",
];

/// `StaffRole`, lowest first
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Role {
    Moderator,
    Admin,
    Owner,
}

impl Role {
    fn id(self) -> &'static str {
        match self {
            Role::Moderator => "moderator",
            Role::Admin => "admin",
            Role::Owner => "owner",
        }
    }

    /// `StaffRole.parse`
    fn parse(text: &str) -> Option<Role> {
        match java_trim(text).to_lowercase().as_str() {
            "moderator" => Some(Role::Moderator),
            "admin" => Some(Role::Admin),
            "owner" => Some(Role::Owner),
            _ => None,
        }
    }
}

/// Why a call isn't answered with a result.
pub(crate) enum Stop {
    Error(i64, String),
    /// a case left to Java (a parameter of an unusual type, a database error)
    Decline,
}

impl From<sqlx::Error> for Stop {
    fn from(e: sqlx::Error) -> Stop {
        warn!("Admin database: {e}");
        Stop::Decline
    }
}

pub(crate) type Answer = Result<Value, Stop>;

pub(crate) fn invalid(message: impl Into<String>) -> Stop {
    Stop::Error(INVALID_PARAMS, message.into())
}

pub struct AdminStores {
    admin: SqlitePool,
    reports: Option<SqlitePool>,
    /// db/support.db and the accounts (db/authorized_user.db), for the support calls
    support: Option<(SqlitePool, PathBuf)>,
    /// db/career.db, for the Career's records
    career: Option<SqlitePool>,
    /// the Career's content, for the calls that need it
    pub(crate) career_content: Option<crate::career::CareerContent>,
    /// the content views Java writes
    pub(crate) career_views: Option<crate::career::CareerViews>,
    /// Java's db/solo.db, next to career.db: the nemesis
    pub(crate) solo: Option<PathBuf>,
    /// the card catalog, as the platform service has it loaded (`Platform::watch` sets it)
    pub(crate) catalog: std::sync::RwLock<Option<std::sync::Arc<crate::catalog::Catalog>>>,
    /// `XMAGE_OWNER` (`-Dxmage.owner`), trimmed; empty when there is none
    owner: String,
}

async fn open(path: &Path, tables: &[&str]) -> anyhow::Result<SqlitePool> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let options =
        SqliteConnectOptions::new().filename(path).create_if_missing(true).busy_timeout(Duration::from_secs(10));
    // one connection: calls take turns, as Java's synchronized stores do
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await?;
    for table in tables {
        sqlx::query(table).execute(&pool).await?;
    }
    Ok(pool)
}

impl AdminStores {
    /// The tables Java hasn't made yet are made as ORMLite makes them. The journal mode stays Java's.
    pub async fn open(admin: &Path, reports: Option<&Path>, owner: &str) -> anyhow::Result<AdminStores> {
        let admin = open(admin, &ADMIN_TABLES).await?;
        let reports = match reports {
            Some(path) => Some(open(path, &REPORT_TABLES).await?),
            None => None,
        };
        Ok(AdminStores {
            admin,
            reports,
            support: None,
            career: None,
            career_content: None,
            career_views: None,
            solo: None,
            catalog: std::sync::RwLock::new(None),
            owner: java_trim(owner).to_string(),
        })
    }

    /// The support calls too, on Java's db/support.db (its tables are Java's, made by its migrations) and the
    /// accounts, which are opened afresh on each call: a rollback to H2 puts the file aside.
    pub async fn add_support(&mut self, support: &Path, accounts: &Path) -> anyhow::Result<()> {
        let options = SqliteConnectOptions::new().filename(support).busy_timeout(Duration::from_secs(10));
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await?;
        self.support = Some((pool, accounts.to_path_buf()));
        Ok(())
    }

    /// The Career's records too, on Java's db/career.db (its tables are Java's)
    pub fn set_catalog(&self, catalog: std::sync::Arc<crate::catalog::Catalog>) {
        *self.catalog.write().unwrap() = Some(catalog);
    }

    pub fn add_career_views(&mut self, views: &Path) {
        self.career_views = Some(crate::career::CareerViews::new(views));
    }

    pub async fn add_career(&mut self, career: &Path, content: Option<&Path>) -> anyhow::Result<()> {
        let options = SqliteConnectOptions::new().filename(career).busy_timeout(Duration::from_secs(10));
        self.career = Some(SqlitePoolOptions::new().max_connections(1).connect_with(options).await?);
        self.solo = Some(career.with_file_name("solo.db"));
        self.career_content = match content {
            Some(dir) => Some(crate::career::CareerContent::load(dir)?),
            None => None,
        };
        Ok(())
    }

    pub fn handles(&self, method: &str) -> bool {
        ADMIN_METHODS.contains(&method)
            || (self.reports.is_some() && REPORT_METHODS.contains(&method))
            || (self.support.is_some() && crate::support::SUPPORT_METHODS.contains(&method))
            || (self.career.is_some() && crate::career::CAREER_METHODS.contains(&method))
    }

    /// `keep`: commit the change (`rust`), or roll it back (`shadow`)
    pub async fn call(&self, caller: &Caller, request: &Request, keep: bool) -> Outcome {
        let method = request.method.as_str();
        // session calls: before login, Java says so
        if method != "inviteCheck" && caller.user.is_none() {
            return Outcome::Declined;
        }
        let mut admin = match self.admin.acquire().await {
            Ok(connection) => connection,
            Err(e) => return Stop::from(e).into(),
        };
        // the second database: reports, or support (with the accounts, read only)
        let mut reports = None;
        let mut accounts = None;
        let support = crate::support::SUPPORT_METHODS.contains(&method);
        let career = crate::career::CAREER_METHODS.contains(&method);
        let second = if REPORT_METHODS.contains(&method) {
            self.reports.as_ref()
        } else if career {
            self.career.as_ref()
        } else if support {
            let Some((pool, path)) = &self.support else { return Outcome::Declined };
            accounts = crate::support::open_read_only(path).await;
            Some(pool)
        } else {
            None
        };
        if REPORT_METHODS.contains(&method) || support || career {
            let Some(pool) = second else { return Outcome::Declined };
            match pool.acquire().await {
                Ok(connection) => reports = Some(connection),
                Err(e) => return Stop::from(e).into(),
            }
        }
        // IMMEDIATE: the write lock up front, so a read followed by a write can't meet Java's lock halfway
        let begin = async |connection: &mut SqliteConnection| sqlx::query("BEGIN IMMEDIATE").execute(connection).await;
        if let Err(e) = begin(&mut admin).await {
            return Stop::from(e).into();
        }
        if let Some(connection) = reports.as_deref_mut() {
            if let Err(e) = begin(connection).await {
                let _ = sqlx::query("ROLLBACK").execute(&mut *admin).await;
                return Stop::from(e).into();
            }
        }
        let mut audits = Vec::new();
        let answer = if career {
            let db = reports.as_deref_mut().expect("acquired");
            self.career(caller, request, &mut admin, db).await
        } else if support {
            let db = reports.as_deref_mut().expect("acquired");
            self.support(caller, request, &mut admin, db, accounts.as_mut(), &mut audits).await
        } else {
            self.run(caller, request, &mut admin, reports.as_deref_mut(), &mut audits).await
        };
        let end = if keep && answer.is_ok() { "COMMIT" } else { "ROLLBACK" };
        let mut ended = sqlx::query(end).execute(&mut *admin).await.is_ok();
        if let Some(connection) = reports.as_deref_mut() {
            ended &= sqlx::query(end).execute(connection).await.is_ok();
        }
        if !ended {
            warn!("Admin database: {end} failed for {method}");
            return Outcome::Declined;
        }
        if keep && answer.is_ok() {
            for line in audits {
                info!("{line}");
            }
        }
        match answer {
            Ok(value) => Outcome::Result(value),
            Err(stop) => stop.into(),
        }
    }

    /// the staff member making this call (`Staff.member`): the actor and role, or None for players
    pub(crate) async fn member(
        &self,
        caller: &Caller,
        db: &mut SqliteConnection,
    ) -> Result<Option<(String, Role)>, Stop> {
        if caller.admin {
            return Ok(Some(("console".into(), Role::Owner)));
        }
        let (true, Some(name)) = (caller.account, &caller.user) else { return Ok(None) };
        if !self.owner.is_empty() && self.owner.to_lowercase() == java_trim(name).to_lowercase() {
            return Ok(Some((name.clone(), Role::Owner)));
        }
        let key = java_trim(name).to_lowercase();
        if key.is_empty() {
            return Ok(None);
        }
        let role: Option<Option<String>> = sqlx::query_scalar("SELECT `role` FROM `admin_staff` WHERE `key` = ?")
            .bind(&key)
            .fetch_optional(&mut *db)
            .await?;
        Ok(role.flatten().as_deref().and_then(Role::parse).map(|role| (name.clone(), role)))
    }

    /// `Staff.require`: the actor for the audit log
    pub(crate) async fn require(
        &self,
        caller: &Caller,
        db: &mut SqliteConnection,
        least: Role,
    ) -> Result<String, Stop> {
        let Some((actor, role)) = self.member(caller, db).await? else {
            return Err(Stop::Error(NOT_AUTHORIZED, "Staff only".into()));
        };
        if role < least {
            return Err(Stop::Error(
                NOT_AUTHORIZED,
                format!("This needs the {} role; yours is {}", least.id(), role.id()),
            ));
        }
        Ok(actor)
    }

    async fn run(
        &self,
        caller: &Caller,
        request: &Request,
        db: &mut SqliteConnection,
        reports: Option<&mut SqliteConnection>,
        audits: &mut Vec<String>,
    ) -> Answer {
        let p = &request.params;
        let now = now_millis();
        match request.method.as_str() {
            "staffMe" => Ok(match self.member(caller, db).await? {
                Some((actor, role)) => json!({"role": role.id(), "actor": actor}),
                None => json!({}),
            }),
            "adminStaff" => {
                // `accounts` is XMAGE_AUTH, which the console's connection doesn't show
                if caller.admin {
                    return Err(Stop::Decline);
                }
                self.require(caller, db, Role::Owner).await?;
                let rows = sqlx::query("SELECT * FROM `admin_staff` ORDER BY `name` ").fetch_all(&mut *db).await?;
                let members: Vec<Value> = rows.iter().map(staff_json).collect();
                Ok(json!({"owner": self.owner, "members": members, "accounts": true}))
            }
            "adminAudit" => {
                self.require(caller, db, Role::Moderator).await?;
                let before = if has(p, 3) { long(p, 3)? } else { 0 };
                let (actor, action, target) = (opt_string(p, 0)?, opt_string(p, 1)?, opt_string(p, 2)?);
                let mut sql = String::from("SELECT * FROM `audit_entry` WHERE `id` > 0 ");
                let mut binds: Vec<String> = Vec::new();
                if let Some(actor) = actor.filter(|a| !is_blank(a)) {
                    sql.push_str("AND `actor` = ? ");
                    binds.push(java_trim(&actor).into());
                }
                if let Some(action) = action.filter(|a| !is_blank(a)) {
                    sql.push_str("AND `action` LIKE ? ");
                    binds.push(format!("{}%", java_trim(&action)));
                }
                if let Some(target) = target.filter(|t| !is_blank(t)) {
                    sql.push_str("AND `target` = ? ");
                    binds.push(java_trim(&target).into());
                }
                if before > 0 {
                    sql.push_str(&format!("AND `id` < {before} "));
                }
                sql.push_str("ORDER BY `id` DESC LIMIT 100 ");
                let mut query = sqlx::query(&sql);
                for bind in &binds {
                    query = query.bind(bind);
                }
                let rows = query.fetch_all(&mut *db).await?;
                Ok(Value::Array(rows.iter().map(audit_json).collect()))
            }
            "adminClientErrors" => {
                self.require(caller, db, Role::Moderator).await?;
                let include_resolved = opt_bool(p, 0, false)?;
                let filter = if include_resolved { "" } else { "WHERE `resolved` = 0 " };
                let sql = format!("SELECT * FROM `admin_client_error` {filter}ORDER BY `lastSeen` DESC LIMIT 200 ");
                let rows = sqlx::query(&sql).fetch_all(&mut *db).await?;
                Ok(Value::Array(rows.iter().map(client_error_json).collect()))
            }
            "adminClientErrorResolve" => {
                let actor = self.require(caller, db, Role::Moderator).await?;
                let id = long(p, 0)?;
                let resolved = opt_bool(p, 1, true)?;
                let Some(row) = sqlx::query("SELECT * FROM `admin_client_error` WHERE `id` = ?")
                    .bind(id)
                    .fetch_optional(&mut *db)
                    .await?
                else {
                    return Err(invalid(format!("There is no client error {id}")));
                };
                let mut group = client_error_json(&row);
                let (by, at) = if resolved { (Some(actor.as_str()), now) } else { (None, 0) };
                // only the columns that change, so a report counted meanwhile isn't overwritten
                if resolved {
                    sqlx::query(
                        "UPDATE `admin_client_error` SET `resolved` = 1, `resolvedBy` = ?, `resolvedAt` = ?, \
                         `regressed` = 0 WHERE `id` = ?",
                    )
                } else {
                    sqlx::query(
                        "UPDATE `admin_client_error` SET `resolved` = 0, `resolvedBy` = ?, `resolvedAt` = ? WHERE `id` = ?",
                    )
                }
                .bind(by)
                .bind(at)
                .bind(id)
                .execute(&mut *db)
                .await?;
                let fields = group.as_object_mut().expect("an object");
                rebuild(fields, "resolved", json!(resolved));
                rebuild(fields, "resolvedBy", by.map_or(Value::Null, |b| json!(b)));
                rebuild(fields, "resolvedAt", json!(at));
                if resolved {
                    rebuild(fields, "regressed", json!(false));
                }
                let message = fields.get("message").and_then(Value::as_str).map(str::to_string);
                let action = if resolved { "clientError.resolve" } else { "clientError.reopen" };
                audit(db, audits, &actor, action, Some(&id.to_string()), message.as_deref(), None, now).await?;
                Ok(group)
            }
            "adminAllowAdd" => {
                let actor = self.require(caller, db, Role::Admin).await?;
                let who = string(p, 0)?;
                let note = opt_string(p, 1)?.unwrap_or_default();
                let who = java_trim(&who);
                if who.is_empty() {
                    return Err(invalid("Type an account name or an email address"));
                }
                if utf16_len(who) > 254 {
                    return Err(invalid("That is too long for a name or an email address"));
                }
                let email = who.contains('@');
                let key = who.to_lowercase();
                let column = if email { "email" } else { "name" };
                let existing = sqlx::query(&format!("SELECT * FROM `allow_entry` WHERE `{column}` = ? "))
                    .bind(&key)
                    .fetch_optional(&mut *db)
                    .await?;
                let entry = match existing {
                    Some(row) => allow_json(&row),
                    None => {
                        let note = cut(Some(&note), MAX_NOTE);
                        let (name, mail) = if email { (None, Some(key.as_str())) } else { (Some(key.as_str()), None) };
                        let id = sqlx::query(
                            "INSERT INTO `allow_entry` (`name` ,`email` ,`note` ,`addedBy` ,`addedAt` ) VALUES (?,?,?,?,?)",
                        )
                        .bind(name)
                        .bind(mail)
                        .bind(note.as_deref())
                        .bind(&actor)
                        .bind(now)
                        .execute(&mut *db)
                        .await?
                        .last_insert_rowid();
                        object([
                            ("id", json!(id)),
                            ("name", name.map_or(Value::Null, |n| json!(n))),
                            ("email", mail.map_or(Value::Null, |m| json!(m))),
                            ("note", note.map_or(Value::Null, |n| json!(n))),
                            ("addedBy", json!(actor)),
                            ("addedAt", json!(now)),
                        ])
                    }
                };
                let target =
                    entry.get("name").or_else(|| entry.get("email")).and_then(Value::as_str).map(str::to_string);
                let note = entry.get("note").and_then(Value::as_str).map(str::to_string);
                audit(db, audits, &actor, "access.allow", target.as_deref(), note.as_deref(), None, now).await?;
                Ok(entry)
            }
            "adminInviteCreate" => {
                let actor = self.require(caller, db, Role::Admin).await?;
                let note = opt_string(p, 0)?.unwrap_or_default();
                let max_uses = opt_int(p, 1, 1)?.clamp(1, MAX_INVITE_USES);
                let days = opt_int(p, 2, 7)?.clamp(1, MAX_INVITE_DAYS);
                let mut bytes = [0u8; 12];
                getrandom::getrandom(&mut bytes).map_err(|_| Stop::Decline)?;
                let code = base64url(&bytes);
                let note = cut(Some(&note), MAX_NOTE).unwrap_or_default();
                let expires_at = now + days * DAY_MILLIS;
                sqlx::query(
                    "INSERT INTO `invite` (`code` ,`note` ,`createdBy` ,`createdAt` ,`expiresAt` ,`maxUses` ,`uses` ,\
                     `revoked` ,`usedBy` ) VALUES (?,?,?,?,?,?,?,?,?)",
                )
                .bind(&code)
                .bind(&note)
                .bind(&actor)
                .bind(now)
                .bind(expires_at)
                .bind(max_uses)
                .bind(0)
                .bind(false)
                .bind("")
                .execute(&mut *db)
                .await?;
                let detail =
                    format!("{max_uses} uses{}", if note.is_empty() { String::new() } else { format!(", {note}") });
                audit(db, audits, &actor, "access.invite.create", Some(&code), Some(&detail), None, now).await?;
                Ok(object([
                    ("code", json!(code)),
                    ("note", json!(note)),
                    ("createdBy", json!(actor)),
                    ("createdAt", json!(now)),
                    ("expiresAt", json!(expires_at)),
                    ("maxUses", json!(max_uses)),
                    ("uses", json!(0)),
                    ("revoked", json!(false)),
                    ("usedBy", json!("")),
                ]))
            }
            "adminInviteRevoke" => {
                let actor = self.require(caller, db, Role::Admin).await?;
                let code = string(p, 0)?;
                let Some(row) = invite(db, &code).await? else { return Ok(json!(false)) };
                if row.get::<Option<i64>, _>("revoked").unwrap_or(0) != 0 {
                    return Ok(json!(false));
                }
                sqlx::query("UPDATE `invite` SET `revoked` = 1 WHERE `code` = ?")
                    .bind(row.get::<Option<String>, _>("code"))
                    .execute(&mut *db)
                    .await?;
                audit(db, audits, &actor, "access.invite.revoke", Some(&code), None, None, now).await?;
                Ok(json!(true))
            }
            "inviteCheck" => {
                let code = string(p, 0)?;
                let reason = match invite(db, &code).await? {
                    None => Err("This invite link doesn't exist. Check that it was copied whole."),
                    Some(row) => {
                        let number = |column: &str| row.get::<Option<i64>, _>(column).unwrap_or(0);
                        if number("revoked") != 0 {
                            Err("This invite link was revoked.")
                        } else if number("uses") >= number("maxUses") {
                            Err("This invite link was used up.")
                        } else if now >= number("expiresAt") {
                            Err("This invite link has expired.")
                        } else {
                            Ok(number("expiresAt"))
                        }
                    }
                };
                Ok(match reason {
                    Ok(expires_at) => json!({"valid": true, "expiresAt": expires_at}),
                    Err(reason) => json!({"valid": false, "reason": reason, "expiresAt": 0}),
                })
            }
            "adminGetReports" => {
                self.require(caller, db, Role::Moderator).await?;
                let open_only = opt_bool(p, 0, true)?;
                let reports = reports.ok_or(Stop::Decline)?;
                let filter = if open_only { "WHERE `open` = 1 " } else { "" };
                let sql = format!("SELECT * FROM `player_report` {filter}ORDER BY `createdAt` DESC LIMIT 200 ");
                let rows = sqlx::query(&sql).fetch_all(&mut *reports).await?;
                Ok(Value::Array(rows.iter().map(report_json).collect()))
            }
            "adminCloseReport" => {
                let actor = self.require(caller, db, Role::Moderator).await?;
                let id = long(p, 0)?;
                let resolution = string(p, 1)?;
                let reports = reports.ok_or(Stop::Decline)?;
                let open: Option<Option<i64>> = sqlx::query_scalar("SELECT `open` FROM `player_report` WHERE `id` = ?")
                    .bind(id)
                    .fetch_optional(&mut *reports)
                    .await?;
                if open.flatten().unwrap_or(0) == 0 {
                    return Ok(json!(false));
                }
                let text = java_trim(&resolution);
                sqlx::query("UPDATE `player_report` SET `open` = 0, `resolution` = ?, `closedAt` = ? WHERE `id` = ?")
                    .bind(utf16_prefix(text, MAX_REASON))
                    .bind(now)
                    .bind(id)
                    .execute(&mut *reports)
                    .await?;
                audit(db, audits, &actor, "report.close", Some(&format!("report {id}")), Some(&resolution), None, now)
                    .await?;
                Ok(json!(true))
            }
            "adminFeedback" => {
                self.require(caller, db, Role::Moderator).await?;
                let status = opt_string(p, 0)?.filter(|s| !is_blank(s));
                let rows = match status {
                    Some(status) => {
                        sqlx::query("SELECT * FROM `admin_feedback` WHERE `status` = ? ORDER BY `id` DESC LIMIT 200 ")
                            .bind(java_trim(&status))
                            .fetch_all(&mut *db)
                            .await?
                    }
                    None => {
                        sqlx::query("SELECT * FROM `admin_feedback` ORDER BY `id` DESC LIMIT 200 ")
                            .fetch_all(&mut *db)
                            .await?
                    }
                };
                Ok(Value::Array(rows.iter().map(feedback_json).collect()))
            }
            "adminFeedbackMark" => {
                let actor = self.require(caller, db, Role::Moderator).await?;
                let id = long(p, 0)?;
                let status = string(p, 1)?;
                let note = if has(p, 2) { Some(string(p, 2)?) } else { None };
                if !["new", "seen", "done"].contains(&status.as_str()) {
                    return Err(invalid("The status is new, seen or done"));
                }
                let Some(row) = sqlx::query("SELECT * FROM `admin_feedback` WHERE `id` = ?")
                    .bind(id)
                    .fetch_optional(&mut *db)
                    .await?
                else {
                    return Err(invalid(format!("There is no feedback {id}")));
                };
                let mut entry = feedback_json(&row);
                let note = cut(note.as_deref(), MAX_NOTE);
                let fields = entry.as_object_mut().expect("an object");
                fields.insert("status".into(), json!(status));
                if let Some(note) = &note {
                    fields.insert("note".into(), json!(note));
                }
                fields.insert("handledBy".into(), json!(actor));
                fields.insert("handledAt".into(), json!(now));
                let note = note.or_else(|| fields.get("note").and_then(Value::as_str).map(str::to_string));
                sqlx::query("UPDATE `admin_feedback` SET `status` = ?, `note` = ?, `handledBy` = ?, `handledAt` = ? WHERE `id` = ?")
                    .bind(&status)
                    .bind(note.as_deref())
                    .bind(&actor)
                    .bind(now)
                    .bind(id)
                    .execute(&mut *db)
                    .await?;
                let action = format!("feedback.{status}");
                let target = format!("feedback {id}");
                audit(db, audits, &actor, &action, Some(&target), note.as_deref(), None, now).await?;
                Ok(entry)
            }
            _ => Err(Stop::Decline),
        }
    }
}

impl From<Stop> for Outcome {
    fn from(stop: Stop) -> Outcome {
        match stop {
            Stop::Error(code, message) => Outcome::error(code, message),
            Stop::Decline => Outcome::Declined,
        }
    }
}

/// `AdminStore.audit`: the entry, and the log line Java writes for it once the change is kept
#[allow(clippy::too_many_arguments)]
pub(crate) async fn audit(
    db: &mut SqliteConnection,
    audits: &mut Vec<String>,
    actor: &str,
    action: &str,
    target: Option<&str>,
    detail: Option<&str>,
    reason: Option<&str>,
    now: i64,
) -> Result<(), Stop> {
    let target = target.unwrap_or("");
    let reason = cut(reason, MAX_REASON);
    sqlx::query(
        "INSERT INTO `audit_entry` (`at` ,`actor` ,`action` ,`target` ,`detail` ,`reason` ) VALUES (?,?,?,?,?,?)",
    )
    .bind(now)
    .bind(actor)
    .bind(action)
    .bind(target)
    .bind(cut(detail, 1000))
    .bind(reason.as_deref())
    .execute(&mut *db)
    .await?;
    audits.push(format!(
        "Audit: {actor} {action}{}{}",
        if target.is_empty() { String::new() } else { format!(" {target}") },
        match reason.as_deref() {
            Some(reason) if !reason.is_empty() => format!(" ({reason})"),
            _ => String::new(),
        }
    ));
    Ok(())
}

async fn invite(db: &mut SqliteConnection, code: &str) -> Result<Option<SqliteRow>, Stop> {
    if is_blank(code) {
        return Ok(None);
    }
    Ok(sqlx::query("SELECT * FROM `invite` WHERE `code` = ?").bind(java_trim(code)).fetch_optional(&mut *db).await?)
}

/// Shadow comparison for these methods: a write answered a moment apart differs in its times, and an invite in its
/// random code.
pub fn difference(method: &str, java: &Value, rust: &Value) -> Option<String> {
    let (times, code): (&[&str], bool) = match method {
        "adminAllowAdd" => (&["addedAt"], false),
        "adminInviteCreate" => (&["createdAt", "expiresAt"], true),
        "adminClientErrorResolve" => (&["resolvedAt"], false),
        "adminFeedbackMark" => (&["handledAt"], false),
        "careerShowProgress" => return crate::career::difference(java, rust),
        _ if crate::support::SUPPORT_METHODS.contains(&method) => {
            return crate::support::difference(method, java, rust)
        }
        _ => (&[], false),
    };
    let (Value::Object(j), Value::Object(r)) = (java, rust) else { return compare::difference(java, rust) };
    let mut rust = r.clone();
    for time in times {
        if let (Some(a), Some(b)) = (j.get(*time).and_then(Value::as_i64), r.get(*time).and_then(Value::as_i64)) {
            if (a - b).abs() <= 60_000 {
                rust.insert((*time).into(), json!(a));
            }
        }
    }
    if code {
        let random = |v: Option<&Value>| v.and_then(Value::as_str).is_some_and(|c| c.len() == 16);
        if random(j.get("code")) && random(r.get("code")) {
            rust.insert("code".into(), j["code"].clone());
        }
    }
    compare::difference(java, &Value::Object(rust))
}

pub(crate) fn now_millis() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Java's `String.trim()`
pub(crate) fn java_trim(text: &str) -> &str {
    text.trim_matches(|c: char| c <= ' ')
}

pub(crate) fn is_blank(text: &str) -> bool {
    java_trim(text).is_empty()
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// the first `max` UTF-16 units, as `substring(0, max)` cuts
pub(crate) fn utf16_prefix(text: &str, max: usize) -> String {
    if utf16_len(text) <= max {
        return text.to_string();
    }
    let units: Vec<u16> = text.encode_utf16().take(max).collect();
    String::from_utf16_lossy(&units)
}

/// `AdminStore.cut`: trimmed, then cut to `max`
pub(crate) fn cut(text: Option<&str>, max: usize) -> Option<String> {
    text.map(|t| utf16_prefix(java_trim(t), max))
}

/// `Base64.getUrlEncoder().withoutPadding()`
fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..=chunk.len() {
            out.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

// --- parameters, as `RpcCall` reads them; anything of another type is left to Java's messages ---

pub(crate) fn has(params: &[Value], index: usize) -> bool {
    params.get(index).is_some_and(|v| !v.is_null())
}

pub(crate) fn string(params: &[Value], index: usize) -> Result<String, Stop> {
    match params.get(index) {
        Some(Value::String(text)) => Ok(text.clone()),
        _ => Err(Stop::Decline),
    }
}

pub(crate) fn opt_string(params: &[Value], index: usize) -> Result<Option<String>, Stop> {
    if has(params, index) {
        string(params, index).map(Some)
    } else {
        Ok(None)
    }
}

pub(crate) fn long(params: &[Value], index: usize) -> Result<i64, Stop> {
    params.get(index).and_then(Value::as_i64).ok_or(Stop::Decline)
}

fn opt_int(params: &[Value], index: usize, default: i64) -> Result<i64, Stop> {
    if !has(params, index) {
        return Ok(default);
    }
    long(params, index).and_then(|n| i32::try_from(n).map(i64::from).map_err(|_| Stop::Decline))
}

pub(crate) fn opt_bool(params: &[Value], index: usize, default: bool) -> Result<bool, Stop> {
    match params.get(index) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(flag)) => Ok(*flag),
        Some(_) => Err(Stop::Decline),
    }
}

// --- rows as Gson writes the entities: fields in declaration order, null strings left out ---

pub(crate) enum Kind {
    Text,
    Number,
    Flag,
}

pub(crate) fn row_json(row: &SqliteRow, columns: &[(&str, Kind)]) -> Value {
    let mut out = Map::new();
    for (column, kind) in columns {
        let value = match kind {
            Kind::Text => match row.try_get::<Option<String>, _>(*column).ok().flatten() {
                Some(text) => json!(text),
                None => continue,
            },
            // a NULL read into a Java primitive is 0 (false)
            Kind::Number => json!(row.try_get::<Option<i64>, _>(*column).ok().flatten().unwrap_or(0)),
            Kind::Flag => json!(row.try_get::<Option<i64>, _>(*column).ok().flatten().unwrap_or(0) != 0),
        };
        out.insert((*column).into(), value);
    }
    Value::Object(out)
}

pub(crate) fn object<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Object(fields.into_iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k.to_string(), v)).collect())
}

/// sets a field of an entity's JSON, keeping the declaration order (a null field is left out)
fn rebuild(fields: &mut Map<String, Value>, key: &str, value: Value) {
    let order = [
        "id",
        "fingerprint",
        "message",
        "source",
        "path",
        "stack",
        "appVersion",
        "userAgent",
        "context",
        "lastUser",
        "count",
        "firstSeen",
        "lastSeen",
        "resolved",
        "resolvedBy",
        "resolvedAt",
        "regressed",
    ];
    if value.is_null() {
        fields.remove(key);
    } else {
        fields.insert(key.into(), value);
    }
    let mut sorted = Map::new();
    for name in order {
        if let Some(v) = fields.remove(name) {
            sorted.insert(name.into(), v);
        }
    }
    *fields = sorted;
}

fn audit_json(row: &SqliteRow) -> Value {
    use Kind::*;
    row_json(
        row,
        &[
            ("id", Number),
            ("at", Number),
            ("actor", Text),
            ("action", Text),
            ("target", Text),
            ("detail", Text),
            ("reason", Text),
        ],
    )
}

fn feedback_json(row: &SqliteRow) -> Value {
    use Kind::*;
    row_json(
        row,
        &[
            ("id", Number),
            ("createdAt", Number),
            ("userName", Text),
            ("type", Text),
            ("title", Text),
            ("message", Text),
            ("context", Text),
            ("status", Text),
            ("note", Text),
            ("handledBy", Text),
            ("handledAt", Number),
        ],
    )
}

fn staff_json(row: &SqliteRow) -> Value {
    use Kind::*;
    row_json(row, &[("key", Text), ("name", Text), ("role", Text), ("grantedBy", Text), ("grantedAt", Number)])
}

fn client_error_json(row: &SqliteRow) -> Value {
    use Kind::*;
    row_json(
        row,
        &[
            ("id", Number),
            ("fingerprint", Text),
            ("message", Text),
            ("source", Text),
            ("path", Text),
            ("stack", Text),
            ("appVersion", Text),
            ("userAgent", Text),
            ("context", Text),
            ("lastUser", Text),
            ("count", Number),
            ("firstSeen", Number),
            ("lastSeen", Number),
            ("resolved", Flag),
            ("resolvedBy", Text),
            ("resolvedAt", Number),
            ("regressed", Flag),
        ],
    )
}

fn allow_json(row: &SqliteRow) -> Value {
    use Kind::*;
    row_json(
        row,
        &[("id", Number), ("name", Text), ("email", Text), ("note", Text), ("addedBy", Text), ("addedAt", Number)],
    )
}

fn report_json(row: &SqliteRow) -> Value {
    use Kind::*;
    row_json(
        row,
        &[
            ("id", Number),
            ("reporter", Text),
            ("reported", Text),
            ("reason", Text),
            ("details", Text),
            ("gameId", Text),
            ("createdAt", Number),
            ("open", Flag),
            ("resolution", Text),
            ("closedAt", Number),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caller(user: &str) -> Caller {
        Caller { user: Some(user.into()), admin: false, account: true, ip: "127.0.0.1".into(), connection: 1 }
    }

    fn console() -> Caller {
        Caller { user: Some("Admin".into()), admin: true, account: false, ip: "127.0.0.1".into(), connection: 2 }
    }

    fn request(method: &str, params: Value) -> Request {
        Request { id: Some(json!(1)), method: method.into(), params: params.as_array().cloned().unwrap_or_default() }
    }

    async fn stores() -> (AdminStores, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("playmat-admin-{}", uuid::Uuid::new_v4()));
        let stores =
            AdminStores::open(&dir.join("web_admin.db"), Some(&dir.join("web_reports.db")), "Boss").await.unwrap();
        (stores, dir)
    }

    fn result(outcome: Outcome) -> Value {
        match outcome {
            Outcome::Result(value) => value,
            other => panic!("expected a result, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn roles_and_refusals() {
        let (stores, dir) = stores().await;
        let me = async |c: &Caller| stores.call(c, &request("staffMe", json!([])), true).await;
        assert_eq!(result(me(&caller("pat")).await), json!({}));
        assert_eq!(result(me(&console()).await), json!({"role": "owner", "actor": "console"}));
        assert_eq!(result(me(&caller(" boss ")).await), json!({"role": "owner", "actor": " boss "}));
        sqlx::query("INSERT INTO admin_staff VALUES ('mod', 'Mod', 'Moderator ', 'Boss', 1)")
            .execute(&stores.admin)
            .await
            .unwrap();
        assert_eq!(result(me(&caller("MOD")).await), json!({"role": "moderator", "actor": "MOD"}));

        let refused = stores.call(&caller("pat"), &request("adminAudit", json!([])), true).await;
        assert_eq!(refused, Outcome::error(NOT_AUTHORIZED, "Staff only"));
        let low = stores.call(&caller("mod"), &request("adminInviteCreate", json!([])), true).await;
        assert_eq!(low, Outcome::error(NOT_AUTHORIZED, "This needs the admin role; yours is moderator"));
        let before_login = Caller { user: None, ..caller("x") };
        assert_eq!(stores.call(&before_login, &request("staffMe", json!([])), true).await, Outcome::Declined);
        let odd = stores.call(&caller("mod"), &request("adminAudit", json!([1])), true).await;
        assert_eq!(odd, Outcome::Declined);

        let staff = result(stores.call(&caller("boss"), &request("adminStaff", json!([])), true).await);
        assert_eq!(staff["owner"], "Boss");
        assert_eq!(staff["members"][0]["key"], "mod");
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn invites_allowlist_and_audit() {
        let (stores, dir) = stores().await;
        let boss = caller("Boss");
        let invite =
            result(stores.call(&boss, &request("adminInviteCreate", json!([" for Sam ", 500, 0])), true).await);
        let code = invite["code"].as_str().unwrap().to_string();
        assert_eq!(code.len(), 16);
        assert_eq!(invite["note"], "for Sam");
        assert_eq!(invite["maxUses"], 100);
        assert_eq!(invite["expiresAt"].as_i64().unwrap() - invite["createdAt"].as_i64().unwrap(), DAY_MILLIS);

        let check = async |code: String| stores.call(&boss, &request("inviteCheck", json!([code])), true).await;
        let ok = result(check(format!(" {code} ")).await);
        assert_eq!(ok, json!({"valid": true, "expiresAt": invite["expiresAt"]}));
        assert_eq!(
            result(stores.call(&boss, &request("adminInviteRevoke", json!([code.clone()])), true).await),
            json!(true)
        );
        assert_eq!(
            result(stores.call(&boss, &request("adminInviteRevoke", json!([code.clone()])), true).await),
            json!(false)
        );
        assert_eq!(result(check(code.clone()).await)["reason"], "This invite link was revoked.");
        assert_eq!(result(check("nope".into()).await)["expiresAt"], 0);

        let added =
            result(stores.call(&boss, &request("adminAllowAdd", json!([" Sam@Example.com ", " hi "])), true).await);
        assert_eq!(added["email"], "sam@example.com");
        assert!(added.get("name").is_none());
        let again = result(stores.call(&boss, &request("adminAllowAdd", json!(["sam@example.com"])), true).await);
        assert_eq!(again["id"], added["id"]);
        let blank = stores.call(&boss, &request("adminAllowAdd", json!(["  "])), true).await;
        assert_eq!(blank, Outcome::error(INVALID_PARAMS, "Type an account name or an email address"));

        // shadow: nothing is kept
        stores.call(&boss, &request("adminAllowAdd", json!(["ghost"])), false).await;
        let log = result(stores.call(&boss, &request("adminAudit", json!([null, "access"])), true).await);
        let actions: Vec<&str> = log.as_array().unwrap().iter().map(|e| e["action"].as_str().unwrap()).collect();
        assert_eq!(actions, ["access.allow", "access.allow", "access.invite.revoke", "access.invite.create"]);
        assert_eq!(log[3]["detail"], "100 uses, for Sam");
        let paged =
            result(stores.call(&boss, &request("adminAudit", json!([null, null, null, log[1]["id"]])), true).await);
        assert_eq!(paged.as_array().unwrap().len(), 2);
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn reports_and_client_errors() {
        let (stores, dir) = stores().await;
        let pool = stores.reports.clone().unwrap();
        sqlx::query("INSERT INTO player_report (reporter, reported, reason, details, createdAt, open) VALUES ('a','b','spam','x',5,1)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO admin_client_error (fingerprint, message, count, firstSeen, lastSeen, resolved, regressed) \
             VALUES ('f', 'boom', 3, 1, 2, 0, 1)",
        )
        .execute(&stores.admin)
        .await
        .unwrap();
        let mod_ = caller("Boss");
        let reports = result(stores.call(&mod_, &request("adminGetReports", json!([])), true).await);
        assert_eq!(reports[0]["open"], true);
        assert!(reports[0].get("gameId").is_none());
        assert_eq!(
            result(stores.call(&mod_, &request("adminCloseReport", json!([1, " done "])), true).await),
            json!(true)
        );
        assert_eq!(
            result(stores.call(&mod_, &request("adminCloseReport", json!([1, "again"])), true).await),
            json!(false)
        );
        let all = result(stores.call(&mod_, &request("adminGetReports", json!([false])), true).await);
        assert_eq!((all[0]["resolution"].as_str(), all[0]["open"].as_bool()), (Some("done"), Some(false)));

        let resolved = result(stores.call(&mod_, &request("adminClientErrorResolve", json!([1])), true).await);
        assert_eq!((resolved["resolved"].as_bool(), resolved["regressed"].as_bool()), (Some(true), Some(false)));
        assert_eq!(resolved["resolvedBy"], "Boss");
        let keys: Vec<&String> = resolved.as_object().unwrap().keys().collect();
        assert_eq!(keys.last().map(|k| k.as_str()), Some("regressed"));
        assert_eq!(result(stores.call(&mod_, &request("adminClientErrors", json!([])), true).await), json!([]));
        let reopened = result(stores.call(&mod_, &request("adminClientErrorResolve", json!([1, false])), true).await);
        assert!(reopened.get("resolvedBy").is_none());
        let missing = stores.call(&mod_, &request("adminClientErrorResolve", json!([9])), true).await;
        assert_eq!(missing, Outcome::error(INVALID_PARAMS, "There is no client error 9"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn helpers() {
        assert_eq!(base64url(&[0xfb, 0xff, 0xbf, 0, 1, 2, 3, 4, 5, 6, 7, 8]), "-_-_AAECAwQFBgcI");
        assert_eq!(cut(Some("  abc  "), 2).as_deref(), Some("ab"));
        assert_eq!(utf16_prefix("a😀b", 2), "a\u{fffd}");
        let java = json!({"code": "AAAAAAAAAAAAAAAA", "createdAt": 1000, "expiresAt": 2000, "note": ""});
        let rust = json!({"code": "BBBBBBBBBBBBBBBB", "createdAt": 1005, "expiresAt": 2005, "note": ""});
        assert_eq!(difference("adminInviteCreate", &java, &rust), None);
        assert!(difference("adminAudit", &java, &rust).is_some());
    }
}
