//! The Career's records that need nothing but db/career.db (Java's `CareerStore`): the collection, the saved deck and
//! what a game paid. The rest of the Career (its content, packs, quests, payouts and the games it starts) stays with
//! Java for now (docs/rust/Career.md).

use crate::admin::{opt_string, AdminStores, Answer, Stop};
use playmat_protocol::handlers::Caller;
use playmat_protocol::jsonrpc::{Request, NOT_AUTHORIZED};
use serde_json::{json, Value};
use sqlx::sqlite::SqliteConnection;
use sqlx::Row;

pub const CAREER_METHODS: [&str; 3] = ["careerCollection", "careerDeck", "careerGameResult"];

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
            _ => Err(Stop::Decline),
        }
    }
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
        stores.add_career(&dir.join("career.db")).await.unwrap();
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
