//! The capsule shop in Rust (`CareerCapsules`, docs/rust/Career.md): the shop's view, buying tokens and pulls.
//! A pull's capsules come from the seeded dice (`dice.rs`), as Java's do, so both sides pull the same ones.

use crate::admin::{invalid, AdminStores, Answer, Stop};
use crate::career::{self, CareerContent};
use playmat_protocol::jsonrpc::Request;
use serde_json::{json, Value};
use sqlx::sqlite::SqliteConnection;
use std::collections::{BTreeMap, HashSet};

/// `CareerCapsules.MAX_PULLS`, `BUNDLES`
const MAX_PULLS: i64 = 10;
const BUNDLES: [i64; 3] = [1, 5, 10];

/// every item a machine holds, in its sets' order: (set, item)
fn items(machine: &Value) -> impl Iterator<Item = (&Value, &Value)> {
    machine["sets"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|set| set["items"].as_array().into_iter().flatten().map(move |item| (set, item)))
}

/// `CareerCapsules.owned`: every machine item the account has, as unlocks of its machine's kind, in their order
async fn owned(db: &mut SqliteConnection, user: &str, machines: &Value) -> Result<Vec<String>, Stop> {
    let mut kinds = std::collections::HashMap::new();
    for machine in machines.as_array().into_iter().flatten() {
        for (_, item) in items(machine) {
            if let Some(id) = item["id"].as_str() {
                kinds.insert(id.to_owned(), machine["kind"].as_str().unwrap_or_default().to_owned());
            }
        }
    }
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT kind, item FROM unlocks WHERE user = ? ORDER BY at, kind, item")
            .bind(user)
            .fetch_all(&mut *db)
            .await?;
    Ok(rows.into_iter().filter(|(kind, item)| kinds.get(item) == Some(kind)).map(|(_, item)| item).collect())
}

/// `CareerService.ensureProfile`
async fn ensure_profile(db: &mut SqliteConnection, user: &str, now: i64) -> Result<(), Stop> {
    sqlx::query("INSERT OR IGNORE INTO profile (user, created, starter, coins) VALUES (?, ?, '', 0)")
        .bind(user)
        .bind(now)
        .execute(&mut *db)
        .await?;
    Ok(())
}

/// `CareerCapsules.addTokens`
async fn add_tokens(db: &mut SqliteConnection, user: &str, amount: i64, reason: &str, now: i64) -> Result<(), Stop> {
    if amount == 0 {
        return Ok(());
    }
    sqlx::query("UPDATE profile SET capsule_tokens = capsule_tokens + ? WHERE user = ?")
        .bind(amount)
        .bind(user)
        .execute(&mut *db)
        .await?;
    sqlx::query("INSERT INTO token_log (user, at, amount, reason) VALUES (?, ?, ?, ?)")
        .bind(user)
        .bind(now)
        .bind(amount)
        .bind(reason)
        .execute(&mut *db)
        .await?;
    Ok(())
}

async fn add_unlock(db: &mut SqliteConnection, user: &str, cosmetic: &Value, now: i64) -> Result<(), Stop> {
    let (Some(kind), Some(id)) = (cosmetic["kind"].as_str(), cosmetic["id"].as_str()) else { return Ok(()) };
    sqlx::query("INSERT OR IGNORE INTO unlocks (user, kind, item, at) VALUES (?, ?, ?, ?)")
        .bind(user)
        .bind(kind)
        .bind(id)
        .bind(now)
        .execute(&mut *db)
        .await?;
    Ok(())
}

/// `call.integer(i)`: a whole number that fits Java's int
fn integer(request: &Request, index: usize) -> Result<i64, Stop> {
    request.params.get(index).and_then(Value::as_i64).filter(|n| i32::try_from(*n).is_ok()).ok_or(Stop::Decline)
}

impl AdminStores {
    pub(crate) async fn capsules(
        &self,
        request: &Request,
        user: &str,
        content: &CareerContent,
        machines: &Value,
        db: &mut SqliteConnection,
    ) -> Answer {
        let now = crate::admin::now_millis();
        let token_price =
            content.rewards.get("capsules").and_then(|c| c.get("tokenPrice")).and_then(Value::as_i64).unwrap_or(120);
        match request.method.as_str() {
            // `CareerCapsules.shop`
            "careerMachines" => {
                ensure_profile(db, user, now).await?;
                let view = career::profile(db, user, content).await?.ok_or(Stop::Decline)?;
                let owned = owned(db, user, machines).await?;
                let rows: Vec<(String, String)> = sqlx::query_as("SELECT card, item FROM card_styles WHERE user = ?")
                    .bind(user)
                    .fetch_all(&mut *db)
                    .await?;
                // a TreeMap: card names in order
                let styles: BTreeMap<String, String> = rows.into_iter().collect();
                Ok(json!({
                    "tokens": view["capsuleTokens"],
                    "coins": view["coins"],
                    "tokenPrice": token_price,
                    "bundles": BUNDLES,
                    "machines": machines,
                    "owned": owned,
                    "styles": styles,
                }))
            }
            // `CareerCapsules.buyTokens`
            "careerBuyTokens" => {
                let count = integer(request, 0)?;
                if !BUNDLES.contains(&count) {
                    return Err(invalid("Tokens come in bundles of [1, 5, 10]."));
                }
                let price = token_price * count;
                ensure_profile(db, user, now).await?;
                let coins: i64 = sqlx::query_scalar("SELECT coins FROM profile WHERE user = ?")
                    .bind(user)
                    .fetch_one(&mut *db)
                    .await?;
                if coins < price {
                    let verb = if count == 1 { " token costs " } else { " tokens cost " };
                    return Err(invalid(format!("{count}{verb}{price} coins; you have {coins}.")));
                }
                sqlx::query("UPDATE profile SET coins = coins + ? WHERE user = ?")
                    .bind(-price)
                    .bind(user)
                    .execute(&mut *db)
                    .await?;
                add_tokens(db, user, count, "bought", now).await?;
                Ok(career::profile(db, user, content).await?.unwrap_or(Value::Null))
            }
            // `CareerCapsules.pull`
            "careerPull" => {
                let Some(Value::String(id)) = request.params.first() else { return Err(Stop::Decline) };
                let count = integer(request, 1)?;
                let (Some(templates), Some(roster)) = (
                    self.career_views.as_ref().and_then(|views| views.view("achievements")),
                    self.career_views.as_ref().and_then(|views| views.view("opponents")),
                ) else {
                    return Err(Stop::Decline);
                };
                let Some(machine) = machines.as_array().into_iter().flatten().find(|m| m["id"] == json!(id)) else {
                    return Err(invalid(format!("There's no machine called {id}.")));
                };
                if !(1..=MAX_PULLS).contains(&count) {
                    return Err(invalid(format!("Pull 1 to {MAX_PULLS} capsules at a time.")));
                }
                ensure_profile(db, user, now).await?;
                let mut owned: HashSet<String> = owned(db, user, machines).await?.into_iter().collect();
                let mut left: Vec<(&Value, &Value)> = items(machine)
                    .filter(|(_, item)| item["id"].as_str().is_some_and(|id| !owned.contains(id)))
                    .collect();
                let name = machine["name"].as_str().unwrap_or_default();
                if left.is_empty() {
                    return Err(invalid(format!("You have every capsule in {name}.")));
                }
                let pulls = count.min(left.len() as i64);
                let tokens: i64 = sqlx::query_scalar("SELECT capsule_tokens FROM profile WHERE user = ?")
                    .bind(user)
                    .fetch_one(&mut *db)
                    .await?;
                if tokens < pulls {
                    let ask = if pulls == 1 {
                        "A pull takes a token".to_owned()
                    } else {
                        format!("{pulls} pulls take {pulls} tokens")
                    };
                    return Err(invalid(format!("{ask}; you have {tokens}.")));
                }
                let pulled: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pulls WHERE user = ? AND machine = ?")
                    .bind(user)
                    .bind(id)
                    .fetch_one(&mut *db)
                    .await?;
                let secret = AdminStores::career_secret(db).await?;
                let mut draw = crate::dice::roll(&secret, &format!("pull/{user}/{id}/{pulled}"));
                add_tokens(db, user, -pulls, &format!("pull {id}"), now).await?;
                let kind = machine["kind"].as_str().unwrap_or_default();
                let mut results = Vec::new();
                for _ in 0..pulls {
                    let (set, item) = left.remove(draw.next_int(left.len() as i32) as usize);
                    let item_id = item["id"].as_str().unwrap_or_default();
                    add_unlock(db, user, &json!({"kind": kind, "id": item_id}), now).await?;
                    sqlx::query("INSERT INTO pulls (user, machine, item, at) VALUES (?, ?, ?, ?)")
                        .bind(user)
                        .bind(id)
                        .bind(item_id)
                        .bind(now)
                        .execute(&mut *db)
                        .await?;
                    owned.insert(item_id.to_owned());
                    let mut pull = serde_json::Map::new();
                    pull.insert("item".into(), json!(item_id));
                    pull.insert("set".into(), set["id"].clone());
                    pull.insert("card".into(), item["card"].clone());
                    let set_done = set["items"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .all(|one| one["id"].as_str().is_some_and(|id| owned.contains(id)));
                    pull.insert("setDone".into(), json!(set_done));
                    if set_done {
                        pull.insert("setReward".into(), set["reward"].clone());
                        add_unlock(db, user, &set["reward"], now).await?;
                        career::add_stat(db, user, "capsule_sets", 1).await?;
                    }
                    let mut rewards = Vec::new();
                    if left.is_empty() {
                        for reward in [&machine["reward"], &machine["title"]] {
                            if reward.is_object() {
                                add_unlock(db, user, reward, now).await?;
                                rewards.push(reward.clone());
                            }
                        }
                        career::add_stat(db, user, "capsule_machines", 1).await?;
                    }
                    pull.insert("machineDone".into(), json!(left.is_empty()));
                    pull.insert("machineRewards".into(), json!(rewards));
                    results.push(Value::Object(pull));
                }
                career::add_stat(db, user, "capsules_pulled", pulls).await?;
                career::achievements_in(db, user, content, &templates, roster, now).await?;
                let tokens: i64 = sqlx::query_scalar("SELECT capsule_tokens FROM profile WHERE user = ?")
                    .bind(user)
                    .fetch_one(&mut *db)
                    .await?;
                Ok(json!({"machine": id, "pulls": results, "tokens": tokens, "left": left.len()}))
            }
            _ => Err(Stop::Decline),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::admin::AdminStores;
    use playmat_protocol::handlers::Caller;
    use playmat_protocol::jsonrpc::{Outcome, Request};
    use serde_json::{json, Map, Value};
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::ConnectOptions;

    /// `CareerPayoutParityTest.CALL_TABLES`
    const CALL_TABLES: [&str; 8] =
        ["profile", "stats", "quests", "achievements", "unlocks", "token_log", "pulls", "card_styles"];

    /// times are each side's own clock
    fn untimed(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                fields.retain(|key, _| !matches!(key.as_str(), "at" | "created" | "assigned"));
                fields.values_mut().for_each(untimed);
            }
            Value::Array(list) => list.iter_mut().for_each(untimed),
            _ => {}
        }
    }

    /// Each economy call Java answered (CareerPayoutParityTest.callsRustReplays) made again in Rust on the rows from
    /// before: the same answer or refusal, and the same rows after.
    #[tokio::test]
    async fn calls_match_java() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/payouts/calls.json");
        let fixture: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let schema: Vec<String> = serde_json::from_value(fixture["schema"].clone()).unwrap();
        let secret = fixture["secret"].as_str().unwrap();
        let content =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../Mage.Server/src/main/resources/career");
        let calls = fixture["calls"].as_array().unwrap();
        assert!(calls.len() > 15);
        let mut problems = Vec::new();
        for call in calls {
            let name = call["name"].as_str().unwrap();
            let dir = std::env::temp_dir().join(format!("playmat-calls-{}", uuid::Uuid::new_v4()));
            let mut stores = AdminStores::open(&dir.join("web_admin.db"), None, "").await.unwrap();
            let mut admin = SqliteConnectOptions::new().filename(dir.join("web_admin.db")).connect().await.unwrap();
            sqlx::query("CREATE TABLE IF NOT EXISTS `admin_setting` (`name` VARCHAR , `value` VARCHAR , `changedBy` VARCHAR , `changedAt` BIGINT , PRIMARY KEY (`name`) )")
                .execute(&mut admin)
                .await
                .unwrap();
            let mut solo = SqliteConnectOptions::new()
                .filename(dir.join("solo.db"))
                .create_if_missing(true)
                .connect()
                .await
                .unwrap();
            sqlx::query("CREATE TABLE user_state (user TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL, updated INTEGER NOT NULL, PRIMARY KEY (user, key))")
                .execute(&mut solo)
                .await
                .unwrap();
            let mut db = SqliteConnectOptions::new()
                .filename(dir.join("career.db"))
                .create_if_missing(true)
                .connect()
                .await
                .unwrap();
            crate::payouts::fill(&mut db, &schema, secret, call["before"].as_object().unwrap()).await.ok().unwrap();
            std::fs::write(dir.join("career-views.json"), fixture["views"].to_string()).unwrap();
            stores.add_career(&dir.join("career.db"), Some(&content)).await.unwrap();
            stores.add_career_views(&dir.join("career-views.json"));
            let ana =
                Caller { user: Some("Ana".into()), admin: false, account: true, ip: "127.0.0.1".into(), connection: 1 };
            let request = Request {
                id: Some(json!(1)),
                method: call["method"].as_str().unwrap().into(),
                params: call["params"].as_array().cloned().unwrap_or_default(),
            };
            match (stores.call(&ana, &request, true).await, call.get("error")) {
                (Outcome::Error { message, .. }, Some(error)) if json!(message) == *error => {}
                (Outcome::Result(mut answer), None) => {
                    let mut java = call["result"].clone();
                    untimed(&mut answer);
                    untimed(&mut java);
                    if answer != java {
                        problems.push(format!("{name}: java {java} rust {answer}"));
                    }
                }
                (other, _) => problems.push(format!("{name}: java {:?} rust {other:?}", call.get("error"))),
            }
            let mut after = Map::new();
            for table in CALL_TABLES {
                let rows: Vec<Value> = sqlx::query(&format!("SELECT * FROM {table} WHERE user = 'ana'"))
                    .fetch_all(&mut db)
                    .await
                    .unwrap()
                    .iter()
                    .map(crate::payouts::row_json)
                    .collect();
                after.insert(table.into(), Value::Array(rows));
            }
            let mut rust = Value::Object(after);
            let mut java = call["after"].clone();
            untimed(&mut rust);
            untimed(&mut java);
            if let Some(difference) = crate::payouts::difference(java.as_object().unwrap(), rust.as_object().unwrap()) {
                problems.push(format!("{name} rows: {difference}"));
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }
}
