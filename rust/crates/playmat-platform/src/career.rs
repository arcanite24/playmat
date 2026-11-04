//! The Career's records that need nothing but db/career.db (Java's `CareerStore`): the collection, the saved deck and
//! what a game paid. The rest of the Career (its content, packs, quests, payouts and the games it starts) stays with
//! Java for now (docs/rust/Career.md).

use crate::admin::{opt_string, AdminStores, Answer, Stop};
use playmat_protocol::handlers::Caller;
use playmat_protocol::jsonrpc::{Request, NOT_AUTHORIZED};
use serde_json::{json, Value};
use sqlx::sqlite::SqliteConnection;
use sqlx::Row;

pub const CAREER_METHODS: [&str; 5] =
    ["careerCollection", "careerDeck", "careerGameResult", "careerShowProgress", "careerLevels"];

/// `CareerRules.MAX_LEVEL`
const MAX_LEVEL: i64 = 100;

/// The Career's content Java ships in its jar (`resources/career`), read from the copy beside it
/// (`XMAGE_PLATFORM_CAREER_CONTENT`): the level track and the renown step.
pub struct CareerContent {
    /// `CareerProgress.levels`, as Gson writes each `CareerLevelReward`
    levels: Vec<Value>,
    /// `CareerRewards.renownXp`
    renown_xp: i64,
}

impl CareerContent {
    pub fn load(dir: &std::path::Path) -> anyhow::Result<CareerContent> {
        let read = |name: &str| -> anyhow::Result<Value> {
            Ok(serde_json::from_str(&std::fs::read_to_string(dir.join(name))?)?)
        };
        let rewards = read("rewards.json")?;
        let renown_xp = rewards.get("renownXp").and_then(Value::as_i64).unwrap_or(5000).max(1);
        // `CareerRewards.Capsules` defaults, unless the file says otherwise
        let capsules = rewards.get("capsules");
        let capsule =
            |field: &str, default: i64| capsules.and_then(|c| c.get(field)).and_then(Value::as_i64).unwrap_or(default);
        let (level_tokens, every) = (capsule("levelTokens", 1), capsule("levelTokensEvery", 5));
        let int = |entry: &Value, field: &str| -> anyhow::Result<i64> {
            match entry.get(field) {
                None | Some(Value::Null) => Ok(0),
                Some(value) => {
                    value.as_i64().ok_or_else(|| anyhow::anyhow!("levels.json: {field} isn't a whole number"))
                }
            }
        };
        let mut levels: Vec<Value> = Vec::new();
        let Value::Array(entries) = read("levels.json")? else { anyhow::bail!("levels.json isn't a list") };
        for entry in &entries {
            let level = int(entry, "level")?;
            let mut tokens = int(entry, "tokens")?;
            if tokens == 0 && every > 0 && level % every == 0 {
                tokens = level_tokens;
            }
            let mut reward = serde_json::Map::new();
            reward.insert("level".into(), json!(level));
            reward.insert("coins".into(), json!(int(entry, "coins")?));
            reward.insert("packs".into(), json!(int(entry, "packs")?));
            if let Some(wildcard) = entry.get("wildcard").and_then(Value::as_str) {
                reward.insert("wildcard".into(), json!(wildcard));
            }
            if let Some(cosmetic) = entry.get("cosmetic").filter(|c| c.is_object()) {
                let mut kept = serde_json::Map::new();
                for field in ["kind", "id"] {
                    if let Some(text) = cosmetic.get(field).and_then(Value::as_str) {
                        kept.insert(field.into(), json!(text));
                    }
                }
                reward.insert("cosmetic".into(), Value::Object(kept));
            }
            reward.insert("tokens".into(), json!(tokens));
            // a LinkedHashMap by level: a level listed again keeps its place and takes the later entry
            match levels.iter_mut().find(|r| r["level"] == json!(level)) {
                Some(slot) => *slot = Value::Object(reward),
                None => levels.push(Value::Object(reward)),
            }
        }
        Ok(CareerContent { levels, renown_xp })
    }
}

/// `CareerRules.xpForLevel`
fn xp_for_level(level: i64) -> i64 {
    250 + 50 * (level - 1)
}

/// `CareerProgress.xpIntoLevel`: the level, the XP into it, and what the next takes (0 at the top)
fn xp_into_level(xp: i64) -> (i64, i64, i64) {
    let (mut level, mut left) = (1, xp);
    while level < MAX_LEVEL && left >= xp_for_level(level) {
        left -= xp_for_level(level);
        level += 1;
    }
    (level, left, if level >= MAX_LEVEL { 0 } else { xp_for_level(level) })
}

/// `CareerRules.renownFor`: a star for every renownXp past the top level
fn renown_for(xp: i64, renown_xp: i64) -> i64 {
    let top: i64 = (1..MAX_LEVEL).map(xp_for_level).sum();
    let past = xp - top;
    if past <= 0 {
        0
    } else {
        past / renown_xp
    }
}

impl AdminStores {
    pub(crate) async fn career(
        &self,
        caller: &Caller,
        request: &Request,
        admin: &mut SqliteConnection,
        db: &mut SqliteConnection,
    ) -> Answer {
        let user = career_user(caller, admin).await?;
        match request.method.as_str() {
            "careerCollection" => {
                let rows = sqlx::query(
                    "SELECT set_code, card_number, name, rarity, count FROM cards WHERE user = ? ORDER BY name, set_code, card_number",
                )
                .bind(&user)
                .fetch_all(&mut *db)
                .await?;
                let mut cards = Vec::new();
                for row in rows {
                    let text = |column: &str| -> Result<Value, Stop> {
                        Ok(row.try_get::<Option<String>, _>(column)?.map_or(Value::Null, Value::from))
                    };
                    let mut card = serde_json::Map::new();
                    for (field, column) in
                        [("name", "name"), ("setCode", "set_code"), ("cardNumber", "card_number"), ("rarity", "rarity")]
                    {
                        let value = text(column)?;
                        if !value.is_null() {
                            card.insert(field.into(), value);
                        }
                    }
                    card.insert("count".into(), json!(row.try_get::<Option<i64>, _>("count")?.unwrap_or(0)));
                    cards.push(Value::Object(card));
                }
                Ok(Value::Array(cards))
            }
            "careerDeck" => {
                let deck: Option<String> = sqlx::query_scalar("SELECT deck FROM decks WHERE user = ?")
                    .bind(&user)
                    .fetch_optional(&mut *db)
                    .await?;
                stored(deck)
            }
            "careerGameResult" => {
                let key = opt_string(&request.params, 0)?.unwrap_or_default();
                let details: Option<String> = if key.is_empty() {
                    sqlx::query_scalar(
                        "SELECT details FROM progress_games WHERE user = ? AND details IS NOT NULL ORDER BY at DESC LIMIT 1",
                    )
                    .bind(&user)
                    .fetch_optional(&mut *db)
                    .await?
                } else {
                    let by = async |game_key: &str, db: &mut SqliteConnection| -> Result<Option<String>, Stop> {
                        let found: Option<Option<String>> =
                            sqlx::query_scalar("SELECT details FROM progress_games WHERE game_key = ? AND user = ?")
                                .bind(game_key)
                                .bind(&user)
                                .fetch_optional(&mut *db)
                                .await?;
                        Ok(found.flatten())
                    };
                    match by(&key, db).await? {
                        Some(found) => Some(found),
                        // claimed under the game id
                        None => by(&format!("{key}/{user}"), db).await?,
                    }
                };
                stored(details)
            }
            "careerShowProgress" => {
                let Some(content) = &self.career_content else { return Err(Stop::Decline) };
                let Some(Value::Bool(show)) = request.params.first() else { return Err(Stop::Decline) };
                // `ensureProfile`: progression starts here if it hadn't
                sqlx::query("INSERT OR IGNORE INTO profile (user, created, starter, coins) VALUES (?, ?, '', 0)")
                    .bind(&user)
                    .bind(crate::admin::now_millis())
                    .execute(&mut *db)
                    .await?;
                sqlx::query("UPDATE profile SET show_progress = ? WHERE user = ?")
                    .bind(i64::from(*show))
                    .bind(&user)
                    .execute(&mut *db)
                    .await?;
                Ok(profile(db, &user, content).await?.unwrap_or(Value::Null))
            }
            "careerLevels" => {
                let Some(content) = &self.career_content else { return Err(Stop::Decline) };
                let Some(view) = profile(db, &user, content).await? else { return Ok(Value::Null) };
                let xp = view["xp"].as_i64().unwrap_or(0);
                let (level, into, next) = xp_into_level(xp);
                let rows = sqlx::query("SELECT kind, item FROM unlocks WHERE user = ? ORDER BY at, kind, item")
                    .bind(&user)
                    .fetch_all(&mut *db)
                    .await?;
                let mut unlocks = Vec::new();
                for row in rows {
                    let mut cosmetic = serde_json::Map::new();
                    for (field, column) in [("kind", "kind"), ("id", "item")] {
                        if let Some(text) = row.try_get::<Option<String>, _>(column)? {
                            cosmetic.insert(field.into(), json!(text));
                        }
                    }
                    unlocks.push(Value::Object(cosmetic));
                }
                Ok(json!({
                    "level": level,
                    "xp": xp,
                    "xpIntoLevel": into,
                    "xpForNext": next,
                    "rewards": content.levels,
                    "unlocks": unlocks,
                }))
            }
            _ => Err(Stop::Decline),
        }
    }
}

/// `CareerStore.profile`, as Gson writes a `CareerProfile`; None without one
async fn profile(db: &mut SqliteConnection, user: &str, content: &CareerContent) -> Result<Option<Value>, Stop> {
    let Some(row) = sqlx::query("SELECT * FROM profile WHERE user = ?").bind(user).fetch_optional(&mut *db).await?
    else {
        return Ok(None);
    };
    let int = |column: &str| -> Result<i64, Stop> { Ok(row.try_get::<Option<i64>, _>(column)?.unwrap_or(0)) };
    let starter: Option<String> = row.try_get("starter")?;
    let xp = int("xp")?;
    let cards: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(count), 0) FROM cards WHERE user = ?")
        .bind(user)
        .fetch_one(&mut *db)
        .await?;
    let mut view = serde_json::Map::new();
    view.insert("created".into(), json!(int("created")?));
    if let Some(starter) = &starter {
        view.insert("starter".into(), json!(starter));
    }
    for (field, column) in [("coins", "coins"), ("xp", "xp")] {
        view.insert(field.into(), json!(int(column)?));
    }
    view.insert("level".into(), json!(xp_into_level(xp).0));
    for (field, column) in [("wins", "wins"), ("losses", "losses")] {
        view.insert(field.into(), json!(int(column)?));
    }
    view.insert(
        "wildcards".into(),
        json!({"common": int("wc_common")?, "uncommon": int("wc_uncommon")?, "rare": int("wc_rare")?, "mythic": int("wc_mythic")?}),
    );
    view.insert("cards".into(), json!(cards));
    view.insert("packTokens".into(), json!(int("pack_tokens")?));
    view.insert("capsuleTokens".into(), json!(int("capsule_tokens")?));
    view.insert("renown".into(), json!(renown_for(xp, content.renown_xp)));
    view.insert("countAiGames".into(), json!(true));
    view.insert("collection".into(), json!(starter.is_some_and(|s| !s.is_empty())));
    view.insert("showProgress".into(), json!(int("show_progress")? == 1));
    Ok(Some(Value::Object(view)))
}

/// Shadow comparison: a profile made by this call is made a moment apart on each side.
pub fn difference(java: &Value, rust: &Value) -> Option<String> {
    let mut rust = rust.clone();
    if let (Some(a), Some(b)) =
        (java.get("created").and_then(Value::as_i64), rust.get("created").and_then(Value::as_i64))
    {
        if (a - b).abs() <= 60_000 {
            rust["created"] = json!(a);
        }
    }
    playmat_protocol::compare::difference(java, &rust)
}

/// `CareerApi.career`: the signed-in account's Career key (`CareerStore.key`), when the server offers Career
async fn career_user(caller: &Caller, admin: &mut SqliteConnection) -> Result<String, Stop> {
    // without accounts Java says there's no Career; names outside ASCII lower differently in Rust
    let (true, false, Some(user)) = (caller.account, caller.admin, &caller.user) else { return Err(Stop::Decline) };
    if !user.is_ascii() || std::env::var("XMAGE_CAREER").is_ok_and(|v| v.trim().eq_ignore_ascii_case("off")) {
        return Err(Stop::Decline);
    }
    let setting: Option<Option<String>> = sqlx::query_scalar("SELECT `value` FROM `admin_setting` WHERE `name` = ?")
        .bind("runtime.career")
        .fetch_optional(&mut *admin)
        .await?;
    if setting.flatten().as_deref() == Some("off") {
        return Err(Stop::Error(NOT_AUTHORIZED, "This server doesn't offer Career".into()));
    }
    Ok(user.to_lowercase())
}

/// a record Java keeps as Gson's JSON and answers as read back: the same JSON, or null
fn stored(json: Option<String>) -> Answer {
    match json {
        None => Ok(Value::Null),
        Some(text) => serde_json::from_str(&text).map_err(|_| Stop::Decline),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use playmat_protocol::jsonrpc::Outcome;
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::ConnectOptions;

    fn request(method: &str, params: Value) -> Request {
        Request { id: Some(json!(1)), method: method.into(), params: params.as_array().cloned().unwrap_or_default() }
    }

    #[tokio::test]
    async fn collection_deck_and_results() {
        let dir = std::env::temp_dir().join(format!("playmat-career-{}", uuid::Uuid::new_v4()));
        let mut stores = AdminStores::open(&dir.join("web_admin.db"), None, "").await.unwrap();
        let mut db = SqliteConnectOptions::new()
            .filename(dir.join("career.db"))
            .create_if_missing(true)
            .connect()
            .await
            .unwrap();
        for sql in [
            "CREATE TABLE cards (user TEXT NOT NULL, set_code TEXT NOT NULL, card_number TEXT NOT NULL, name TEXT NOT NULL, \
             rarity TEXT NOT NULL, count INTEGER NOT NULL, PRIMARY KEY (user, set_code, card_number))",
            "CREATE TABLE decks (user TEXT PRIMARY KEY, deck TEXT NOT NULL, updated INTEGER NOT NULL)",
            "CREATE TABLE profile (user TEXT PRIMARY KEY, created INTEGER NOT NULL, starter TEXT NOT NULL, \
             coins INTEGER NOT NULL DEFAULT 0, xp INTEGER NOT NULL DEFAULT 0, wins INTEGER NOT NULL DEFAULT 0, \
             losses INTEGER NOT NULL DEFAULT 0, wc_common INTEGER NOT NULL DEFAULT 0, wc_uncommon INTEGER NOT NULL DEFAULT 0, \
             wc_rare INTEGER NOT NULL DEFAULT 0, wc_mythic INTEGER NOT NULL DEFAULT 0, pack_tokens INTEGER NOT NULL DEFAULT 0, \
             show_progress INTEGER NOT NULL DEFAULT 1, capsule_tokens INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE unlocks (user TEXT NOT NULL, kind TEXT NOT NULL, item TEXT NOT NULL, at INTEGER NOT NULL)",
            "CREATE TABLE progress_games (game_key TEXT PRIMARY KEY, user TEXT NOT NULL, at INTEGER NOT NULL, details TEXT)",
            "INSERT INTO cards VALUES ('neri', 'M21', '2', 'Zap', 'common', 3), ('neri', 'M21', '1', 'Arrow', 'rare', 1), \
             ('bob', 'M21', '3', 'Bolt', 'common', 4)",
            "INSERT INTO decks VALUES ('neri', '{\"name\":\"Red\",\"cards\":[{\"cardName\":\"Zap\",\"amount\":3}]}', 1)",
            "INSERT INTO progress_games VALUES ('t1', 'neri', 10, '{\"coins\":25}'), ('g2/neri', 'neri', 20, '{\"coins\":5}'), \
             ('t3', 'neri', 30, NULL)",
        ] {
            sqlx::query(sql).execute(&mut db).await.unwrap();
        }
        let mut admin = SqliteConnectOptions::new().filename(dir.join("web_admin.db")).connect().await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS `admin_setting` (`name` VARCHAR , `value` VARCHAR , `changedBy` VARCHAR , `changedAt` BIGINT , PRIMARY KEY (`name`) )")
            .execute(&mut admin)
            .await
            .unwrap();
        stores.add_career(&dir.join("career.db"), Some(&content_dir())).await.unwrap();
        let neri =
            Caller { user: Some("Neri".into()), admin: false, account: true, ip: "127.0.0.1".into(), connection: 1 };
        let call = async |method: &str, params: Value| match stores.call(&neri, &request(method, params), true).await {
            Outcome::Result(value) => value,
            other => panic!("{method}: {other:?}"),
        };
        let collection = call("careerCollection", json!([])).await;
        assert_eq!(
            collection,
            json!([
                {"name": "Arrow", "setCode": "M21", "cardNumber": "1", "rarity": "rare", "count": 1},
                {"name": "Zap", "setCode": "M21", "cardNumber": "2", "rarity": "common", "count": 3},
            ])
        );
        assert_eq!(call("careerDeck", json!([])).await["cards"][0]["amount"], json!(3));
        assert_eq!(call("careerGameResult", json!([])).await, json!({"coins": 5}));
        assert_eq!(call("careerGameResult", json!(["t1"])).await, json!({"coins": 25}));
        assert_eq!(call("careerGameResult", json!(["g2"])).await, json!({"coins": 5}));
        assert_eq!(call("careerGameResult", json!(["t3"])).await, Value::Null);

        // progression: the profile is made on the first ask
        assert_eq!(call("careerLevels", json!([])).await, Value::Null);
        let profile = call("careerShowProgress", json!([false])).await;
        assert_eq!(profile["showProgress"], json!(false));
        assert_eq!(
            (profile["level"].clone(), profile["cards"].clone(), profile["collection"].clone()),
            (json!(1), json!(4), json!(false))
        );
        sqlx::query("UPDATE profile SET xp = 260 WHERE user = 'neri'").execute(&mut db).await.unwrap();
        sqlx::query("INSERT INTO unlocks VALUES ('neri', 'avatar', 'hare', 5)").execute(&mut db).await.unwrap();
        let track = call("careerLevels", json!([])).await;
        assert_eq!(
            (track["level"].clone(), track["xpIntoLevel"].clone(), track["xpForNext"].clone()),
            (json!(2), json!(10), json!(300))
        );
        assert_eq!(track["unlocks"], json!([{"kind": "avatar", "id": "hare"}]));
        assert!(track["rewards"].as_array().unwrap().len() > 90);

        // staff switched Career off
        sqlx::query("INSERT INTO admin_setting VALUES ('runtime.career', 'off', 'boss', 1)")
            .execute(&mut admin)
            .await
            .unwrap();
        assert!(matches!(
            stores.call(&neri, &request("careerCollection", json!([])), true).await,
            Outcome::Error { message, .. } if message == "This server doesn't offer Career"
        ));
        std::fs::remove_dir_all(dir).ok();
    }

    fn content_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../Mage.Server/src/main/resources/career")
    }

    #[test]
    fn levels_as_java_counts_them() {
        assert_eq!(xp_into_level(0), (1, 0, 250));
        assert_eq!(xp_into_level(249), (1, 249, 250));
        assert_eq!(xp_into_level(250), (2, 0, 300));
        let top: i64 = (1..MAX_LEVEL).map(xp_for_level).sum();
        assert_eq!(xp_into_level(top + 10), (100, 10, 0));
        assert_eq!(renown_for(top + 4_999, 5000), 0);
        assert_eq!(renown_for(top + 10_000, 5000), 2);
        let content = CareerContent::load(&content_dir()).unwrap();
        assert_eq!(content.renown_xp, 5000);
        assert_eq!(content.levels[0], json!({"level": 2, "coins": 25, "packs": 0, "tokens": 0}));
        assert_eq!(content.levels[1]["cosmetic"], json!({"kind": "avatar", "id": "hare"}));
        // every fifth level gives a capsule token unless the file says otherwise
        assert_eq!(content.levels.iter().find(|l| l["level"] == json!(5)).unwrap()["tokens"], json!(1));
    }

    #[test]
    fn stored_records() {
        assert_eq!(stored(None).ok(), Some(Value::Null));
        assert_eq!(
            stored(Some(r#"{"name":"Mono red","cards":[]}"#.into())).ok(),
            Some(json!({"name": "Mono red", "cards": []}))
        );
        assert!(matches!(stored(Some("{".into())), Err(Stop::Decline)));
    }
}
