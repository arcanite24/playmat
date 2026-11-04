//! Web client decks per account (`deckSyncList`, `deckSyncGet`, `deckSyncPut`, `deckSyncDelete`), on the same
//! SQLite file and table as Java's `DeckStore` (db/web_decks.db), with the same rules: last write wins by the client's
//! change time, deletions stay as tombstones for 90 days, 500 decks of 256 KB per account.
//!
//! Each call runs in one transaction. In `shadow` the transaction is rolled back, because Java makes the change.

use playmat_protocol::handlers::Caller;
use playmat_protocol::jsonrpc::{Outcome, Request, INVALID_PARAMS, NOT_AUTHORIZED, SERVER_REJECTED};
use serde_json::{json, Map, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection, SqlitePool, SqlitePoolOptions};
use sqlx::{Connection, Row};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::warn;

pub const METHODS: [&str; 4] = ["deckSyncList", "deckSyncGet", "deckSyncPut", "deckSyncDelete"];

const MAX_DECKS: i64 = 500;
const MAX_DECK_CHARS: usize = 256 * 1024;
const TOMBSTONE_MILLIS: i64 = 90 * 24 * 60 * 60 * 1000;

pub struct DeckStore {
    pool: SqlitePool,
}

/// An answer that isn't a result: an RPC error, as Java's `AccountApi.decks` maps store errors.
type Failed = (i64, String);

impl DeckStore {
    /// Opens (or creates) the file. The table is created as ORMLite creates it, in case Java hasn't yet. The journal
    /// mode stays Java's.
    pub async fn open(path: &Path) -> anyhow::Result<DeckStore> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        let options =
            SqliteConnectOptions::new().filename(path).create_if_missing(true).busy_timeout(Duration::from_secs(10));
        // one connection: calls take turns, as Java's synchronized store does
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS `web_deck` (`key` VARCHAR , `owner` VARCHAR , `deckId` VARCHAR , \
             `name` VARCHAR , `updatedAt` BIGINT , `deleted` BOOLEAN , `data` TEXT , PRIMARY KEY (`key`) )",
        )
        .execute(&pool)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS `web_deck_owner_idx` ON `web_deck` ( `owner` )").execute(&pool).await?;
        Ok(DeckStore { pool })
    }

    /// `keep`: commit the change (`rust`), or roll it back (`shadow`)
    pub async fn call(&self, caller: &Caller, request: &Request, keep: bool) -> Outcome {
        match self.run(caller, request, keep).await {
            Ok(value) => Outcome::Result(value),
            Err((code, message)) => Outcome::error(code, message),
        }
    }

    async fn run(&self, caller: &Caller, request: &Request, keep: bool) -> Result<Value, Failed> {
        let owner = account(caller)?;
        let params = &request.params;
        // parameters are read before the store is, in Java's order
        let call = match request.method.as_str() {
            "deckSyncList" => Call::List,
            "deckSyncGet" => Call::Get { deck: string(params, 0)? },
            "deckSyncPut" => {
                let updated_at = long(params, 2)?;
                Call::Put { deck: string(params, 0)?, name: string(params, 1)?, updated_at, data: string(params, 3)? }
            }
            "deckSyncDelete" => {
                let deleted_at = long(params, 1)?;
                Call::Delete { deck: string(params, 0)?, deleted_at }
            }
            _ => return Err((playmat_protocol::jsonrpc::METHOD_NOT_FOUND, "Not a deck method".into())),
        };
        let mut connection = self.pool.acquire().await.map_err(store_error)?;
        // IMMEDIATE: the write lock up front, so a read followed by a write can't meet Java's lock halfway
        sqlx::query("BEGIN IMMEDIATE").execute(&mut *connection).await.map_err(store_error)?;
        let result = call.run(&mut connection, &owner.to_lowercase()).await;
        let end = if keep && result.is_ok() { "COMMIT" } else { "ROLLBACK" };
        if let Err(e) = sqlx::query(end).execute(&mut *connection).await {
            warn!("decks {end}: {e}");
            // a connection left inside a transaction mustn't go back to the pool
            let _ = connection.detach().close().await;
            if result.is_ok() {
                return Err(store_error(e));
            }
        }
        result
    }
}

enum Call {
    List,
    Get { deck: String },
    Put { deck: String, name: String, updated_at: i64, data: String },
    Delete { deck: String, deleted_at: i64 },
}

/// a stored row: (name, updatedAt, deleted)
type Existing = Option<(Option<String>, i64, bool)>;

impl Call {
    async fn run(self, db: &mut SqliteConnection, owner: &str) -> Result<Value, Failed> {
        match self {
            Call::List => {
                let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
                sqlx::query("DELETE FROM `web_deck` WHERE `owner` = ? AND `deleted` = 1 AND `updatedAt` < ?")
                    .bind(owner)
                    .bind(now - TOMBSTONE_MILLIS)
                    .execute(&mut *db)
                    .await
                    .map_err(store_error)?;
                let rows =
                    sqlx::query("SELECT `deckId`, `name`, `updatedAt`, `deleted` FROM `web_deck` WHERE `owner` = ?")
                        .bind(owner)
                        .fetch_all(&mut *db)
                        .await
                        .map_err(store_error)?;
                let entries =
                    rows.iter().map(|row| entry(row.get(0), row.get(1), row.get(2), flag(row, 3), None)).collect();
                Ok(Value::Array(entries))
            }
            Call::Get { deck } => {
                let key = format!("{owner}/{}", check_id(&deck)?);
                let row = sqlx::query(
                    "SELECT `deckId`, `name`, `updatedAt`, `deleted`, `data` FROM `web_deck` WHERE `key` = ?",
                )
                .bind(&key)
                .fetch_optional(&mut *db)
                .await
                .map_err(store_error)?;
                Ok(match row {
                    Some(row) if !flag(&row, 3) => entry(row.get(0), row.get(1), row.get(2), false, row.get(4)),
                    _ => Value::Null,
                })
            }
            Call::Put { deck, name, updated_at, data } => {
                check_id(&deck)?;
                if data.is_empty() {
                    return Err((INVALID_PARAMS, "The deck is empty".into()));
                }
                // Java counts UTF-16 units
                let chars = data.encode_utf16().count();
                if chars > MAX_DECK_CHARS {
                    return Err((INVALID_PARAMS, format!("The deck is too large to store ({} KB)", chars / 1024)));
                }
                let key = format!("{owner}/{deck}");
                let existing = existing(db, &key).await?;
                if let Some((_, stored, _)) = &existing {
                    if *stored > updated_at {
                        return Ok(put_result(false, *stored));
                    }
                }
                if existing.as_ref().is_none_or(|(_, _, deleted)| *deleted) {
                    let live: i64 =
                        sqlx::query_scalar("SELECT COUNT(*) FROM `web_deck` WHERE `owner` = ? AND `deleted` = 0")
                            .bind(owner)
                            .fetch_one(&mut *db)
                            .await
                            .map_err(store_error)?;
                    if live >= MAX_DECKS {
                        return Err((
                            INVALID_PARAMS,
                            format!("An account can keep up to {MAX_DECKS} decks on the server"),
                        ));
                    }
                }
                let name = utf16_prefix(&name, 200);
                store(db, existing.is_some(), &key, owner, &deck, Some(name), updated_at, false, Some(&data)).await?;
                Ok(put_result(true, updated_at))
            }
            Call::Delete { deck, deleted_at } => {
                let key = format!("{owner}/{}", check_id(&deck)?);
                let existing = existing(db, &key).await?;
                if let Some((_, stored, _)) = &existing {
                    if *stored > deleted_at {
                        return Ok(put_result(false, *stored));
                    }
                }
                // a deletion keeps the name it had (none for a deck the server never saw)
                let name = existing.as_ref().and_then(|(name, _, _)| name.as_deref());
                store(db, existing.is_some(), &key, owner, &deck, name, deleted_at, true, None).await?;
                Ok(put_result(true, deleted_at))
            }
        }
    }
}

async fn existing(db: &mut SqliteConnection, key: &str) -> Result<Existing, Failed> {
    let row = sqlx::query("SELECT `name`, `updatedAt`, `deleted` FROM `web_deck` WHERE `key` = ?")
        .bind(key)
        .fetch_optional(&mut *db)
        .await
        .map_err(store_error)?;
    Ok(row.map(|row| (row.get(0), row.get(1), flag(&row, 2))))
}

/// ORMLite's createOrUpdate: an update keeps the row (and so its place in the list)
#[allow(clippy::too_many_arguments)]
async fn store(
    db: &mut SqliteConnection,
    exists: bool,
    key: &str,
    owner: &str,
    deck: &str,
    name: Option<&str>,
    updated_at: i64,
    deleted: bool,
    data: Option<&str>,
) -> Result<(), Failed> {
    let sql = if exists {
        "UPDATE `web_deck` SET `owner` = ?, `deckId` = ?, `name` = ?, `updatedAt` = ?, `deleted` = ?, `data` = ? \
         WHERE `key` = ?"
    } else {
        "INSERT INTO `web_deck` (`owner`, `deckId`, `name`, `updatedAt`, `deleted`, `data`, `key`) \
         VALUES (?, ?, ?, ?, ?, ?, ?)"
    };
    sqlx::query(sql)
        .bind(owner)
        .bind(deck)
        .bind(name)
        .bind(updated_at)
        .bind(deleted)
        .bind(data)
        .bind(key)
        .execute(&mut *db)
        .await
        .map_err(store_error)?;
    Ok(())
}

/// `DeckSyncEntry` as Gson writes it: no `name` or `data` when they are null
fn entry(id: Option<String>, name: Option<String>, updated_at: i64, deleted: bool, data: Option<String>) -> Value {
    let mut out = Map::new();
    if let Some(id) = id {
        out.insert("id".into(), Value::from(id));
    }
    if let Some(name) = name {
        out.insert("name".into(), Value::from(name));
    }
    out.insert("updatedAt".into(), Value::from(updated_at));
    out.insert("deleted".into(), Value::from(deleted));
    if let Some(data) = data {
        out.insert("data".into(), Value::from(data));
    }
    Value::Object(out)
}

fn put_result(stored: bool, updated_at: i64) -> Value {
    json!({ "stored": stored, "updatedAt": updated_at })
}

/// booleans as ORMLite stores them in SQLite (1 or 0)
fn flag(row: &sqlx::sqlite::SqliteRow, index: usize) -> bool {
    row.try_get::<i64, _>(index).map(|v| v != 0).unwrap_or(false)
}

/// `AccountApi.account`: a signed-in user, on a server with accounts
fn account(caller: &Caller) -> Result<&str, Failed> {
    let Some(user) = caller.user.as_deref().filter(|_| !caller.admin) else {
        return Err((NOT_AUTHORIZED, "Sign in first".into()));
    };
    if !caller.account {
        return Err((NOT_AUTHORIZED, "This server has no accounts, so decks stay in your browser".into()));
    }
    Ok(user)
}

/// `RpcCall.string`: any JSON primitive, as text
fn string(params: &[Value], index: usize) -> Result<String, Failed> {
    match params.get(index) {
        None | Some(Value::Null) => Err((INVALID_PARAMS, format!("Missing parameter #{index}"))),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(Value::Number(n)) => Ok(n.to_string()),
        Some(Value::Bool(b)) => Ok(b.to_string()),
        Some(_) => Err((INVALID_PARAMS, format!("Parameter #{index} must be a string"))),
    }
}

/// `AccountApi.longValue`: a number (cut to a whole one), or a string holding a whole number
fn long(params: &[Value], index: usize) -> Result<i64, Failed> {
    match params.get(index) {
        None | Some(Value::Null) => Err((INVALID_PARAMS, format!("Missing parameter #{index}"))),
        Some(value) => gson_long(value).ok_or_else(|| (INVALID_PARAMS, format!("Parameter #{index} must be a number"))),
    }
}

/// Gson's `getAsLong`, which also takes an array of exactly one such value
fn gson_long(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Value::String(s) => s.parse::<i64>().ok(),
        Value::Array(items) if items.len() == 1 => gson_long(&items[0]),
        _ => None,
    }
}

/// `DeckStore.checkId`: [A-Za-z0-9_:.-]{1,64}
fn check_id(deck: &str) -> Result<&str, Failed> {
    let ok = (1..=64).contains(&deck.len())
        && deck.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b':' | b'.' | b'-'));
    if ok {
        Ok(deck)
    } else {
        Err((INVALID_PARAMS, "Bad deck id".into()))
    }
}

/// Java's `substring(0, units)`, in whole characters
fn utf16_prefix(text: &str, units: usize) -> &str {
    let mut count = 0;
    for (at, c) in text.char_indices() {
        count += c.len_utf16();
        if count > units {
            return &text[..at];
        }
    }
    text
}

fn store_error(e: sqlx::Error) -> Failed {
    warn!("decks: {e}");
    (SERVER_REJECTED, "Decks can't be stored on this server right now".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caller(user: &str) -> Caller {
        Caller { user: Some(user.into()), admin: false, account: true, ip: "127.0.0.1".into(), connection: 1 }
    }

    fn request(method: &str, params: Vec<Value>) -> Request {
        Request { id: Some(Value::from(1)), method: method.into(), params }
    }

    async fn result(store: &DeckStore, caller: &Caller, method: &str, params: Vec<Value>, keep: bool) -> Value {
        match store.call(caller, &request(method, params), keep).await {
            Outcome::Result(v) => v,
            other => panic!("{method}: {other:?}"),
        }
    }

    fn code(outcome: Outcome) -> i64 {
        match outcome {
            Outcome::Error { code, .. } => code,
            other => panic!("expected an error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn last_write_wins_and_shadow_changes_nothing() {
        let dir = std::env::temp_dir().join(format!("playmat-decks-{}", uuid::Uuid::new_v4()));
        let store = DeckStore::open(&dir.join("web_decks.db")).await.unwrap();
        let ana = caller("Ana");
        // change times near now: older tombstones are dropped
        let t0 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let put = |id: &str, at: i64| vec![json!(id), json!("Burn"), json!(at), json!("{\"cards\":[]}")];

        assert_eq!(result(&store, &ana, "deckSyncPut", put("d1", t0 + 10), true).await, put_result(true, t0 + 10));
        assert_eq!(result(&store, &ana, "deckSyncPut", put("d1", t0 + 5), true).await, put_result(false, t0 + 10));
        // shadow answers as a write would, and leaves the deck as it was
        assert_eq!(result(&store, &ana, "deckSyncPut", put("d1", t0 + 20), false).await, put_result(true, t0 + 20));
        assert_eq!(
            result(&store, &ana, "deckSyncDelete", vec![json!("d1"), json!(t0 + 30)], false).await,
            put_result(true, t0 + 30)
        );
        let got = result(&store, &ana, "deckSyncGet", vec![json!("d1")], true).await;
        assert_eq!(got["updatedAt"], json!(t0 + 10));
        assert_eq!(got["data"], json!("{\"cards\":[]}"));

        // other accounts don't see it; names are matched ignoring case
        assert_eq!(result(&store, &caller("Bo"), "deckSyncList", vec![], true).await, json!([]));
        assert_eq!(result(&store, &caller("ANA"), "deckSyncList", vec![], true).await.as_array().unwrap().len(), 1);

        assert_eq!(
            result(&store, &ana, "deckSyncDelete", vec![json!("d1"), json!((t0 + 40).to_string())], true).await,
            put_result(true, t0 + 40)
        );
        assert_eq!(result(&store, &ana, "deckSyncGet", vec![json!("d1")], true).await, Value::Null);
        let list = result(&store, &ana, "deckSyncList", vec![], true).await;
        assert_eq!(list, json!([{ "id": "d1", "name": "Burn", "updatedAt": t0 + 40, "deleted": true }]));
        // a deletion of a deck the server never had leaves a tombstone without a name
        let now = t0 + 50;
        result(&store, &ana, "deckSyncDelete", vec![json!("d2"), json!(now)], true).await;
        let list = result(&store, &ana, "deckSyncList", vec![], true).await;
        assert_eq!(list[1], json!({ "id": "d2", "updatedAt": now, "deleted": true }));
        // tombstones older than 90 days go when the list is asked for
        result(&store, &ana, "deckSyncDelete", vec![json!("d3"), json!(1)], true).await;
        let list = result(&store, &ana, "deckSyncList", vec![], true).await;
        assert_eq!(list.as_array().unwrap().len(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn refuses_what_java_refuses() {
        let dir = std::env::temp_dir().join(format!("playmat-decks-{}", uuid::Uuid::new_v4()));
        let store = DeckStore::open(&dir.join("web_decks.db")).await.unwrap();
        let ana = caller("Ana");
        let store = &store;
        let ana = &ana;
        let call = |method: &'static str, params: Vec<Value>| async move {
            store.call(ana, &request(method, params), true).await
        };
        assert_eq!(code(call("deckSyncGet", vec![json!("bad id")]).await), INVALID_PARAMS);
        assert_eq!(code(call("deckSyncGet", vec![]).await), INVALID_PARAMS);
        assert_eq!(code(call("deckSyncGet", vec![json!(["x"])]).await), INVALID_PARAMS);
        assert_eq!(
            code(call("deckSyncPut", vec![json!("d"), json!("n"), json!("soon"), json!("x")]).await),
            INVALID_PARAMS
        );
        assert_eq!(code(call("deckSyncPut", vec![json!("d"), json!("n"), json!(1), json!("")]).await), INVALID_PARAMS);
        let big = "x".repeat(MAX_DECK_CHARS + 1);
        assert_eq!(code(call("deckSyncPut", vec![json!("d"), json!("n"), json!(1), json!(big)]).await), INVALID_PARAMS);

        let guest = Caller { user: None, ..ana.clone() };
        assert_eq!(code(store.call(&guest, &request("deckSyncList", vec![]), true).await), NOT_AUTHORIZED);
        let no_accounts = Caller { account: false, ..ana.clone() };
        assert_eq!(code(store.call(&no_accounts, &request("deckSyncList", vec![]), true).await), NOT_AUTHORIZED);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn names_are_cut_like_java() {
        assert_eq!(utf16_prefix("abc", 2), "ab");
        assert_eq!(utf16_prefix("a😀b", 2), "a");
        assert_eq!(utf16_prefix("a😀b", 3), "a😀");
    }
}
