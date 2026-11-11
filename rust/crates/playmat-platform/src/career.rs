//! The Career's records that need nothing but db/career.db (Java's `CareerStore`): the collection, the saved deck and
//! what a game paid. The rest of the Career (its content, packs, quests, payouts and the games it starts) stays with
//! Java for now (docs/rust/Career.md).

use crate::admin::{opt_string, AdminStores, Answer, Stop};
use playmat_protocol::handlers::Caller;
use playmat_protocol::jsonrpc::{Request, NOT_AUTHORIZED};
use serde_json::{json, Value};
use sqlx::sqlite::SqliteConnection;
use sqlx::Row;

pub const CAREER_METHODS: [&str; 18] = [
    "careerCollection",
    "careerDeck",
    "careerGameResult",
    "careerShowProgress",
    "careerLevels",
    "careerStarters",
    "careerCosmetics",
    "careerOpponents",
    "careerAchievements",
    "careerQuests",
    "careerEquipStyle",
    "careerCraft",
    "careerStart",
    "careerSaveDeck",
    "careerRerollQuest",
    "careerMachines",
    "careerBuyTokens",
    "careerPull",
];

/// `CareerRules.START_COINS`
const START_COINS: i64 = 100;

/// `CareerRules.PLAYSET`
const PLAYSET: i64 = 4;
/// `CareerService.BASIC_NAMES`
const BASIC_NAMES: [&str; 12] = [
    "Plains",
    "Island",
    "Swamp",
    "Mountain",
    "Forest",
    "Wastes",
    "Snow-Covered Plains",
    "Snow-Covered Island",
    "Snow-Covered Swamp",
    "Snow-Covered Mountain",
    "Snow-Covered Forest",
    "Snow-Covered Wastes",
];

/// `CareerRules.rarityOf`, with basics as lands (`CareerService.toCareerCard`)
fn career_rarity(rarity: &str, name: &str, basic: bool) -> &'static str {
    if basic || BASIC_NAMES.contains(&name) {
        return "land";
    }
    match rarity {
        "LAND" => "land",
        "UNCOMMON" => "uncommon",
        "RARE" | "SPECIAL" | "BONUS" => "rare",
        "MYTHIC" => "mythic",
        _ => "common",
    }
}

/// `CareerRules.WINS_TO_UNLOCK_NEXT_TIER`
pub(crate) const WINS_TO_UNLOCK_NEXT_TIER: i64 = 3;

/// The content views Java writes (`CareerViewsExport`, `XMAGE_CAREER_EXPORT`): read again when the file changes,
/// as it does when staff reload the content packs.
pub struct CareerViews {
    path: std::path::PathBuf,
    loaded: std::sync::Mutex<Option<(std::time::SystemTime, std::sync::Arc<Value>)>>,
}

impl CareerViews {
    pub fn new(path: &std::path::Path) -> CareerViews {
        CareerViews { path: path.to_path_buf(), loaded: std::sync::Mutex::new(None) }
    }

    /// one view, or None while Java hasn't written the file (Java answers then)
    pub(crate) fn view(&self, name: &str) -> Option<Value> {
        let modified = std::fs::metadata(&self.path).and_then(|m| m.modified()).ok()?;
        let mut loaded = self.loaded.lock().unwrap();
        if loaded.as_ref().is_none_or(|(at, _)| *at != modified) {
            let views: Value = serde_json::from_str(&std::fs::read_to_string(&self.path).ok()?).ok()?;
            *loaded = Some((modified, std::sync::Arc::new(views)));
        }
        loaded.as_ref().and_then(|(_, views)| views.get(name).cloned())
    }
}

/// `CareerRules.MAX_LEVEL`
pub(crate) const MAX_LEVEL: i64 = 100;

/// The Career's content Java ships in its jar (`resources/career`), read from the copy beside it
/// (`XMAGE_PLATFORM_CAREER_CONTENT`): the level track and the renown step.
pub struct CareerContent {
    /// `CareerProgress.levels`, as Gson writes each `CareerLevelReward`
    pub(crate) levels: Vec<Value>,
    /// `CareerRewards.renownXp`
    pub(crate) renown_xp: i64,
    /// rewards.json as it is (`CareerRewards.File`), for payouts
    pub(crate) rewards: Value,
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
        Ok(CareerContent { levels, renown_xp, rewards })
    }
}

/// `CareerRules.xpForLevel`
pub(crate) fn xp_for_level(level: i64) -> i64 {
    250 + 50 * (level - 1)
}

/// `CareerProgress.xpIntoLevel`: the level, the XP into it, and what the next takes (0 at the top)
pub(crate) fn xp_into_level(xp: i64) -> (i64, i64, i64) {
    let (mut level, mut left) = (1, xp);
    while level < MAX_LEVEL && left >= xp_for_level(level) {
        left -= xp_for_level(level);
        level += 1;
    }
    (level, left, if level >= MAX_LEVEL { 0 } else { xp_for_level(level) })
}

/// `CareerRules.renownFor`: a star for every renownXp past the top level
pub(crate) fn renown_for(xp: i64, renown_xp: i64) -> i64 {
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
            "careerStarters" | "careerCosmetics" => {
                let name = if request.method == "careerStarters" { "starters" } else { "cosmetics" };
                self.career_views.as_ref().and_then(|views| views.view(name)).ok_or(Stop::Decline)
            }
            "careerOpponents" => {
                let Some(roster) = self.career_views.as_ref().and_then(|views| views.view("opponents")) else {
                    return Err(Stop::Decline);
                };
                let wins = wins_by_opponent(db, &user).await?;
                // `CareerLadder.nemesis`: Java's solo.db keeps it under the name as signed in
                let nemesis = match &self.solo {
                    Some(solo) => nemesis(solo, caller.user.as_deref().unwrap_or_default()).await?,
                    None => return Err(Stop::Decline),
                };
                opponents(roster, &wins, nemesis.as_deref()).ok_or(Stop::Decline)
            }
            "careerAchievements" => {
                let views = self.career_views.as_ref();
                let (Some(content), Some(templates), Some(roster)) = (
                    &self.career_content,
                    views.and_then(|views| views.view("achievements")),
                    views.and_then(|views| views.view("opponents")),
                ) else {
                    return Err(Stop::Decline);
                };
                let rows = sqlx::query("SELECT id, at FROM achievements WHERE user = ? ORDER BY at")
                    .bind(&user)
                    .fetch_all(&mut *db)
                    .await?;
                let mut earned = std::collections::HashMap::new();
                for row in rows {
                    earned.insert(row.try_get::<String, _>(0)?, row.try_get::<i64, _>(1)?);
                }
                let totals = totals(db, &user, content, roster).await?;
                let mut views = Vec::new();
                for template in templates.as_array().ok_or(Stop::Decline)? {
                    let total = template["scope"] == json!("total");
                    let counter = template["counter"].as_str().and_then(|counter| totals.get(counter));
                    let progress = if total { counter.copied().unwrap_or(0) } else { 0 };
                    views.push(achievement(template, &earned, progress).ok_or(Stop::Decline)?);
                }
                Ok(Value::Array(views))
            }
            "careerQuests" | "careerRerollQuest" => {
                let Some(templates) = self.career_views.as_ref().and_then(|views| views.view("quests")) else {
                    return Err(Stop::Decline);
                };
                let slot = if request.method == "careerRerollQuest" {
                    // `call.integer(0)`
                    let Some(slot) =
                        request.params.first().and_then(Value::as_i64).filter(|s| i32::try_from(*s).is_ok())
                    else {
                        return Err(Stop::Decline);
                    };
                    Some(slot)
                } else {
                    None
                };
                let reroll_day: Option<Option<String>> =
                    sqlx::query_scalar("SELECT reroll_day FROM profile WHERE user = ?")
                        .bind(&user)
                        .fetch_optional(&mut *db)
                        .await?;
                let Some(reroll_day) = reroll_day else {
                    return match slot {
                        Some(_) => Err(crate::admin::invalid("Open a Career first.")),
                        None => Ok(Value::Null),
                    };
                };
                let (today, week) = utc_day_and_week(crate::admin::now_millis());
                let secret = crate::admin::AdminStores::career_secret(db).await?;
                let book = crate::payouts::Book {
                    content: self.career_content.as_ref().ok_or(Stop::Decline)?,
                    quests: &templates,
                    achievements: &Value::Null,
                    roster: &Value::Null,
                    cosmetics: &Value::Null,
                    secret: &secret,
                };
                let mut reroll_day = reroll_day;
                match slot {
                    // a new day or week picks quests with the seeded dice, as Java does (`refreshQuests`)
                    None => crate::payouts::refresh_quests(db, &book, &user, &today, &week).await?,
                    // `CareerProgress.reroll`
                    Some(slot) => {
                        if reroll_day.as_deref() == Some(today.as_str()) {
                            return Err(crate::admin::invalid("You've already swapped a quest today."));
                        }
                        let rows: Vec<(i64, String)> =
                            sqlx::query_as("SELECT slot, quest FROM quests WHERE user = ? ORDER BY slot")
                                .bind(&user)
                                .fetch_all(&mut *db)
                                .await?;
                        if !rows.iter().any(|(s, _)| *s == slot) {
                            return Err(crate::admin::invalid("There's no quest in that slot."));
                        }
                        if slot == 3 {
                            return Err(crate::admin::invalid("The weekly quest can't be swapped."));
                        }
                        let present = rows.into_iter().map(|(_, quest)| quest).collect();
                        let Some(next) = crate::payouts::pick_quest(
                            &book,
                            &present,
                            false,
                            &format!("reroll/{user}/{today}/{slot}"),
                        ) else {
                            return Err(crate::admin::invalid("There's no other quest to swap in."));
                        };
                        sqlx::query(
                            "INSERT OR REPLACE INTO quests (user, slot, quest, progress, assigned) VALUES (?, ?, ?, 0, ?)",
                        )
                        .bind(&user)
                        .bind(slot)
                        .bind(&next)
                        .bind(&today)
                        .execute(&mut *db)
                        .await?;
                        sqlx::query("UPDATE profile SET reroll_day = ? WHERE user = ?")
                            .bind(&today)
                            .bind(&user)
                            .execute(&mut *db)
                            .await?;
                        reroll_day = Some(today.clone());
                    }
                }
                let mut quests = Vec::new();
                for row in sqlx::query("SELECT slot, quest, progress FROM quests WHERE user = ? ORDER BY slot")
                    .bind(&user)
                    .fetch_all(&mut *db)
                    .await?
                {
                    let id: String = row.try_get(1)?;
                    // a quest removed from the catalogue waits silently until rerolled
                    let Some(template) = templates.as_array().into_iter().flatten().find(|t| t["id"] == json!(id))
                    else {
                        continue;
                    };
                    quests.push(quest(template, row.try_get(0)?, row.try_get(2)?));
                }
                Ok(
                    json!({"quests": quests, "canReroll": reroll_day.as_deref() != Some(today.as_str()), "today": today}),
                )
            }
            "careerMachines" | "careerBuyTokens" | "careerPull" => {
                let (Some(content), Some(machines)) =
                    (&self.career_content, self.career_views.as_ref().and_then(|views| views.view("machines")))
                else {
                    return Err(Stop::Decline);
                };
                self.capsules(request, &user, content, &machines, db).await
            }
            "careerEquipStyle" => {
                let Some(styles) = self.career_views.as_ref().and_then(|views| views.view("styles")) else {
                    return Err(Stop::Decline);
                };
                let Some(Value::String(card)) = request.params.first() else { return Err(Stop::Decline) };
                let item = match request.params.get(1) {
                    None | Some(Value::Null) => None,
                    Some(Value::String(item)) => Some(item.as_str()).filter(|item| !item.is_empty()),
                    Some(_) => return Err(Stop::Decline),
                };
                if card.is_empty() {
                    return Err(crate::admin::invalid("Which card?"));
                }
                if let Some(item) = item {
                    let style = &styles[item];
                    // an item without a card isn't a style: Java fails on it, so it answers
                    if style.is_object() && !style["card"].is_string() {
                        return Err(Stop::Decline);
                    }
                    if style["card"].as_str() != Some(card.as_str()) {
                        return Err(crate::admin::invalid(format!("That style isn't for {card}.")));
                    }
                    let owned: Option<i64> =
                        sqlx::query_scalar("SELECT 1 FROM unlocks WHERE user = ? AND kind = ? AND item = ?")
                            .bind(&user)
                            .bind(style["kind"].as_str().unwrap_or_default())
                            .bind(item)
                            .fetch_optional(&mut *db)
                            .await?;
                    if owned.is_none() {
                        return Err(crate::admin::invalid("Pull that style from its machine first."));
                    }
                    sqlx::query("INSERT OR REPLACE INTO card_styles (user, card, item) VALUES (?, ?, ?)")
                        .bind(&user)
                        .bind(card)
                        .bind(item)
                        .execute(&mut *db)
                        .await?;
                } else {
                    sqlx::query("DELETE FROM card_styles WHERE user = ? AND card = ?")
                        .bind(&user)
                        .bind(card)
                        .execute(&mut *db)
                        .await?;
                }
                // a TreeMap: card names in order
                let rows = sqlx::query("SELECT card, item FROM card_styles WHERE user = ?")
                    .bind(&user)
                    .fetch_all(&mut *db)
                    .await?;
                let mut worn = std::collections::BTreeMap::new();
                for row in rows {
                    worn.insert(row.try_get::<String, _>(0)?, json!(row.try_get::<String, _>(1)?));
                }
                Ok(Value::Object(worn.into_iter().collect()))
            }
            "careerCraft" => {
                let (Some(Value::String(set)), Some(Value::String(number))) =
                    (request.params.first(), request.params.get(1))
                else {
                    return Err(Stop::Decline);
                };
                let (Some(content), Some(templates), Some(roster), Some(catalog)) = (
                    &self.career_content,
                    self.career_views.as_ref().and_then(|views| views.view("achievements")),
                    self.career_views.as_ref().and_then(|views| views.view("opponents")),
                    self.catalog.read().unwrap().clone(),
                ) else {
                    return Err(Stop::Decline);
                };
                let Some((name, set_code, card_number, rarity, basic)) =
                    catalog.printing(set, number).await.map_err(|_| Stop::Decline)?
                else {
                    return Err(crate::admin::invalid(format!("There is no card {set} {number}.")));
                };
                let rarity = career_rarity(&rarity, &name, basic);
                if rarity == "land" {
                    return Err(crate::admin::invalid("Basic lands are free; there's nothing to craft."));
                }
                let wildcard = format!("wc_{rarity}");
                let row = sqlx::query(&format!("SELECT coins, {wildcard} FROM profile WHERE user = ?"))
                    .bind(&user)
                    .fetch_optional(&mut *db)
                    .await?;
                let Some(row) = row else { return Err(crate::admin::invalid("Open a Career first.")) };
                let (coins, wildcards): (i64, i64) = (row.try_get(0)?, row.try_get(1)?);
                let owned: i64 =
                    sqlx::query_scalar("SELECT COALESCE(SUM(count), 0) FROM cards WHERE user = ? AND name = ?")
                        .bind(&user)
                        .bind(&name)
                        .fetch_one(&mut *db)
                        .await?;
                if owned >= PLAYSET {
                    return Err(crate::admin::invalid(format!("You already have {PLAYSET} copies of {name}.")));
                }
                let price = match rarity {
                    "common" => 25,
                    "uncommon" => 50,
                    _ => 0,
                };
                // a common or uncommon wildcard (levels and campaign choices pay them) is spent before coins
                if price > 0 && wildcards <= 0 {
                    if coins < price {
                        return Err(crate::admin::invalid(format!(
                            "{name} costs {price} coins to craft; you have {coins}."
                        )));
                    }
                    sqlx::query("UPDATE profile SET coins = coins + ? WHERE user = ?")
                        .bind(-price)
                        .bind(&user)
                        .execute(&mut *db)
                        .await?;
                } else {
                    if price == 0 && wildcards < 1 {
                        return Err(crate::admin::invalid(format!("{name} takes a {rarity} wildcard; you have none.")));
                    }
                    sqlx::query(&format!("UPDATE profile SET {wildcard} = {wildcard} + ? WHERE user = ?"))
                        .bind(-1)
                        .bind(&user)
                        .execute(&mut *db)
                        .await?;
                }
                sqlx::query(
                    "INSERT INTO cards (user, set_code, card_number, name, rarity, count) VALUES (?, ?, ?, ?, ?, ?) \
                     ON CONFLICT (user, set_code, card_number) DO UPDATE SET count = count + excluded.count",
                )
                .bind(&user)
                .bind(&set_code)
                .bind(&card_number)
                .bind(&name)
                .bind(rarity)
                .bind(1)
                .execute(&mut *db)
                .await?;
                add_stat(db, &user, "cards_crafted", 1).await?;
                achievements_in(db, &user, content, &templates, roster, crate::admin::now_millis()).await?;
                Ok(profile(db, &user, content).await?.unwrap_or(Value::Null))
            }
            "careerStart" => {
                let (Some(content), Some(Value::String(starter))) = (&self.career_content, request.params.first())
                else {
                    return Err(Stop::Decline);
                };
                let Some(starters) = self.career_views.as_ref().and_then(|views| views.view("starterCards")) else {
                    return Err(Stop::Decline);
                };
                let known = self.career_views.as_ref().and_then(|views| views.view("starters")).ok_or(Stop::Decline)?;
                if !known.as_array().into_iter().flatten().any(|s| s["id"] == json!(starter)) {
                    return Err(crate::admin::invalid(format!("There is no starter deck {starter}.")));
                }
                // a deck Java couldn't read isn't in the file: Java says why
                let Some(cards) = starters[starter.as_str()].as_array() else { return Err(Stop::Decline) };
                // progression made the profile with the account's first counted game; picking a starter opens the collection
                let created =
                    sqlx::query("INSERT OR IGNORE INTO profile (user, created, starter, coins) VALUES (?, ?, ?, ?)")
                        .bind(&user)
                        .bind(crate::admin::now_millis())
                        .bind(starter)
                        .bind(START_COINS)
                        .execute(&mut *db)
                        .await?
                        .rows_affected()
                        == 1;
                let opened = created
                    || sqlx::query("UPDATE profile SET starter = ?, coins = coins + ? WHERE user = ? AND starter = ''")
                        .bind(starter)
                        .bind(START_COINS)
                        .bind(&user)
                        .execute(&mut *db)
                        .await?
                        .rows_affected()
                        == 1;
                if !opened {
                    return Err(crate::admin::invalid("You already have a Career."));
                }
                for card in cards {
                    let text = |field: &str| card[field].as_str().ok_or(Stop::Decline);
                    sqlx::query(
                        "INSERT INTO cards (user, set_code, card_number, name, rarity, count) VALUES (?, ?, ?, ?, ?, ?) \
                         ON CONFLICT (user, set_code, card_number) DO UPDATE SET count = count + excluded.count",
                    )
                    .bind(&user)
                    .bind(text("setCode")?)
                    .bind(text("cardNumber")?)
                    .bind(text("name")?)
                    .bind(text("rarity")?)
                    .bind(card["amount"].as_i64().ok_or(Stop::Decline)?)
                    .execute(&mut *db)
                    .await?;
                }
                Ok(profile(db, &user, content).await?.unwrap_or(Value::Null))
            }
            "careerSaveDeck" => {
                let Some(deck) = request.params.first().and_then(gson_deck) else { return Err(Stop::Decline) };
                let starter: Option<String> = sqlx::query_scalar("SELECT starter FROM profile WHERE user = ?")
                    .bind(&user)
                    .fetch_optional(&mut *db)
                    .await?;
                if starter.is_none_or(|starter| starter.is_empty()) {
                    return Err(crate::admin::invalid("Open your Career collection first."));
                }
                // `ownershipProblems`: copies by name, main deck and sideboard together, basics free
                let mut needed: Vec<(String, i64)> = Vec::new();
                for info in deck["cards"].as_array().into_iter().chain(deck["sideboard"].as_array()).flatten() {
                    let name = info["cardName"].as_str().ok_or(Stop::Decline)?;
                    if BASIC_NAMES.contains(&name) {
                        continue;
                    }
                    let amount = info["amount"].as_i64().unwrap_or(0);
                    match needed.iter_mut().find(|(n, _)| n == name) {
                        Some((_, total)) => *total += amount,
                        None => needed.push((name.to_owned(), amount)),
                    }
                }
                let mut problems = Vec::new();
                for (name, amount) in &needed {
                    let owned: i64 =
                        sqlx::query_scalar("SELECT COALESCE(SUM(count), 0) FROM cards WHERE user = ? AND name = ?")
                            .bind(&user)
                            .bind(name)
                            .fetch_one(&mut *db)
                            .await?;
                    if owned < *amount {
                        problems.push(format!("{name}: {amount} in the deck, {owned} owned"));
                    }
                }
                if !problems.is_empty() {
                    return Err(crate::admin::invalid(format!(
                        "The deck has cards you don't own: {}.",
                        problems.join("; ")
                    )));
                }
                sqlx::query("INSERT OR REPLACE INTO decks (user, deck, updated) VALUES (?, ?, ?)")
                    .bind(&user)
                    .bind(deck.to_string())
                    .bind(crate::admin::now_millis())
                    .execute(&mut *db)
                    .await?;
                Ok(json!(true))
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
pub(crate) async fn profile(
    db: &mut SqliteConnection,
    user: &str,
    content: &CareerContent,
) -> Result<Option<Value>, Stop> {
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
/// `CareerQuest`, in its fields' order, nulls left out
pub(crate) fn quest(template: &Value, slot: i64, progress: i64) -> Value {
    let mut view = serde_json::Map::new();
    view.insert("slot".into(), json!(slot));
    for field in ["id", "text", "counts"] {
        if !template[field].is_null() {
            view.insert(field.into(), template[field].clone());
        }
    }
    view.insert("weekly".into(), json!(template["weekly"].as_bool().unwrap_or(false)));
    view.insert("progress".into(), json!(progress));
    for field in ["target", "coins", "xp"] {
        view.insert(field.into(), json!(template[field].as_i64().unwrap_or(0)));
    }
    Value::Object(view)
}

/// `CareerProgress.day` and `week` (UTC): "2026-11-04" and the ISO week, "2026-W45"
pub(crate) fn utc_day_and_week(millis: i64) -> (String, String) {
    let days = millis.div_euclid(86_400_000);
    let (year, month, day) = crate::support::civil(days);
    // ISO weeks start on Monday and belong to the year of their Thursday
    let thursday = days - (days + 3).rem_euclid(7) + 3;
    let (iso_year, _, _) = crate::support::civil(thursday);
    let week = (thursday - crate::support::days_from_civil(iso_year, 1, 1)) / 7 + 1;
    (format!("{year:04}-{month:02}-{day:02}"), format!("{iso_year}-W{week:02}"))
}

/// A `DeckCardLists` as Gson reads it and writes it back: its fields in order, unknown ones dropped, nulls left out.
/// None for anything Gson would read differently (a field of another type, an explicit null list), left to Java.
fn gson_deck(deck: &Value) -> Option<Value> {
    let deck = deck.as_object()?;
    let text =
        |object: &serde_json::Map<String, Value>, field: &str, out: &mut serde_json::Map<String, Value>| match object
            .get(field)
        {
            None | Some(Value::Null) => Some(()),
            Some(Value::String(text)) => {
                out.insert(field.into(), json!(text));
                Some(())
            }
            Some(_) => None,
        };
    let info = |value: &Value| -> Option<Value> {
        let object = value.as_object()?;
        let mut out = serde_json::Map::new();
        for field in ["cardName", "setCode", "cardNumber"] {
            text(object, field, &mut out)?;
        }
        let amount = match object.get("amount") {
            None | Some(Value::Null) => 0,
            Some(value) => value.as_i64().filter(|a| i32::try_from(*a).is_ok())?,
        };
        out.insert("amount".into(), json!(amount));
        Some(Value::Object(out))
    };
    let list = |field: &str| -> Option<Value> {
        match deck.get(field) {
            None => Some(json!([])),
            Some(Value::Array(items)) => items.iter().map(info).collect::<Option<Vec<_>>>().map(Value::Array),
            Some(_) => None,
        }
    };
    let layout = |field: &str, out: &mut serde_json::Map<String, Value>| -> Option<()> {
        match deck.get(field) {
            None | Some(Value::Null) => Some(()),
            Some(Value::Object(layout)) => {
                let mut copy = serde_json::Map::new();
                if let Some(rows) = layout.get("cards").filter(|rows| !rows.is_null()) {
                    let rows = rows.as_array()?;
                    let mut grid = Vec::new();
                    for row in rows {
                        let mut columns = Vec::new();
                        for column in row.as_array()? {
                            columns
                                .push(Value::Array(column.as_array()?.iter().map(info).collect::<Option<Vec<_>>>()?));
                        }
                        grid.push(Value::Array(columns));
                    }
                    copy.insert("cards".into(), Value::Array(grid));
                }
                text(layout, "settings", &mut copy)?;
                out.insert(field.into(), Value::Object(copy));
                Some(())
            }
            Some(_) => None,
        }
    };
    let mut out = serde_json::Map::new();
    text(deck, "name", &mut out)?;
    text(deck, "author", &mut out)?;
    out.insert("cards".into(), list("cards")?);
    out.insert("sideboard".into(), list("sideboard")?);
    layout("cardLayout", &mut out)?;
    layout("sideboardLayout", &mut out)?;
    // a card without a name is counted under null by Java: left to it
    let named =
        |field: &str| out[field].as_array().is_some_and(|cards| cards.iter().all(|c| c["cardName"].is_string()));
    (named("cards") && named("sideboard")).then_some(Value::Object(out))
}

/// `CareerStore.addStat`
pub(crate) async fn add_stat(db: &mut SqliteConnection, user: &str, counter: &str, amount: i64) -> Result<(), Stop> {
    sqlx::query(
        "INSERT INTO stats (user, counter, value) VALUES (?, ?, ?) \
         ON CONFLICT (user, counter) DO UPDATE SET value = value + excluded.value",
    )
    .bind(user)
    .bind(counter)
    .bind(amount)
    .execute(&mut *db)
    .await?;
    Ok(())
}

/// `CareerProgress.totals`: the stats, the level, opponents beaten and the highest tier open
pub(crate) async fn totals(
    db: &mut SqliteConnection,
    user: &str,
    content: &CareerContent,
    roster: Value,
) -> Result<std::collections::HashMap<String, i64>, Stop> {
    let mut totals = std::collections::HashMap::new();
    for row in sqlx::query("SELECT counter, value FROM stats WHERE user = ?").bind(user).fetch_all(&mut *db).await? {
        totals.insert(row.try_get::<String, _>(0)?, row.try_get::<i64, _>(1)?);
    }
    let level = profile(db, user, content).await?.and_then(|view| view["level"].as_i64()).unwrap_or(1);
    totals.insert("level".into(), level);
    let wins = wins_by_opponent(db, user).await?;
    let beaten = roster
        .as_array()
        .ok_or(Stop::Decline)?
        .iter()
        .filter(|entry| entry["view"]["id"].as_str().and_then(|id| wins.get(id)).is_some_and(|won| *won > 0));
    totals.insert("opponents_beaten".into(), beaten.count() as i64);
    let opened = opponents(roster, &wins, None).ok_or(Stop::Decline)?;
    let tiers_open = opened
        .as_array()
        .into_iter()
        .flatten()
        .filter(|view| view["unlocked"] == json!(true))
        .filter_map(|view| view["tier"].as_i64())
        .fold(1, i64::max);
    totals.insert("tiers_open".into(), tiers_open);
    Ok(totals)
}

/// `CareerProgress.achievementsIn` outside a game (no game counters, no result to fill): every lifetime achievement
/// and tier the totals now reach is recorded and paid, its coins, XP, cosmetic and capsule tokens
pub(crate) async fn achievements_in(
    db: &mut SqliteConnection,
    user: &str,
    content: &CareerContent,
    templates: &Value,
    roster: Value,
    now: i64,
) -> Result<(), Stop> {
    let mut earned = std::collections::HashSet::new();
    for id in sqlx::query_scalar::<_, String>("SELECT id FROM achievements WHERE user = ?")
        .bind(user)
        .fetch_all(&mut *db)
        .await?
    {
        earned.insert(id);
    }
    let totals = totals(db, user, content, roster).await?;
    for template in templates.as_array().ok_or(Stop::Decline)? {
        let id = template["id"].as_str().ok_or(Stop::Decline)?;
        let value = if template["scope"] == json!("game") {
            0
        } else {
            template["counter"].as_str().and_then(|counter| totals.get(counter)).copied().unwrap_or(0)
        };
        let tiers = template["tiers"].as_array().map(Vec::as_slice).unwrap_or_default();
        if tiers.is_empty() {
            let target = template["target"].as_i64().unwrap_or(0);
            if earned.contains(id) || value < target || !add_achievement(db, user, id, now).await? {
                continue;
            }
            pay(db, user, template, now).await?;
            continue;
        }
        for (index, step) in tiers.iter().enumerate() {
            let key = format!("{id}:{}", index + 1);
            if earned.contains(&key) {
                continue;
            }
            if value < step["target"].as_i64().unwrap_or(0) || !add_achievement(db, user, &key, now).await? {
                break;
            }
            earned.insert(key.clone());
            pay(db, user, step, now).await?;
            let tokens = step["tokens"].as_i64().unwrap_or(0);
            if tokens > 0 {
                // `CareerCapsules.addTokens`
                sqlx::query("UPDATE profile SET capsule_tokens = capsule_tokens + ? WHERE user = ?")
                    .bind(tokens)
                    .bind(user)
                    .execute(&mut *db)
                    .await?;
                sqlx::query("INSERT INTO token_log (user, at, amount, reason) VALUES (?, ?, ?, ?)")
                    .bind(user)
                    .bind(now)
                    .bind(tokens)
                    .bind(format!("achievement {key}"))
                    .execute(&mut *db)
                    .await?;
            }
        }
    }
    Ok(())
}

/// `CareerStore.addAchievement`: true when it's new
pub(crate) async fn add_achievement(db: &mut SqliteConnection, user: &str, id: &str, now: i64) -> Result<bool, Stop> {
    let done = sqlx::query("INSERT OR IGNORE INTO achievements (user, id, at) VALUES (?, ?, ?)")
        .bind(user)
        .bind(id)
        .bind(now)
        .execute(&mut *db)
        .await?;
    Ok(done.rows_affected() == 1)
}

/// `CareerProgress.pay`: an achievement's (or tier's) coins, XP and cosmetic
pub(crate) async fn pay(db: &mut SqliteConnection, user: &str, reward: &Value, now: i64) -> Result<(), Stop> {
    let (coins, xp) = (reward["coins"].as_i64().unwrap_or(0), reward["xp"].as_i64().unwrap_or(0));
    if coins > 0 {
        sqlx::query("UPDATE profile SET coins = coins + ? WHERE user = ?")
            .bind(coins)
            .bind(user)
            .execute(&mut *db)
            .await?;
    }
    if xp > 0 {
        sqlx::query("UPDATE profile SET xp = xp + ? WHERE user = ?").bind(xp).bind(user).execute(&mut *db).await?;
    }
    if let (Some(kind), Some(item)) = (reward["cosmetic"]["kind"].as_str(), reward["cosmetic"]["id"].as_str()) {
        sqlx::query("INSERT OR IGNORE INTO unlocks (user, kind, item, at) VALUES (?, ?, ?, ?)")
            .bind(user)
            .bind(kind)
            .bind(item)
            .bind(now)
            .execute(&mut *db)
            .await?;
    }
    Ok(())
}

/// `CareerStore.winsByOpponent`
pub(crate) async fn wins_by_opponent(
    db: &mut SqliteConnection,
    user: &str,
) -> Result<std::collections::HashMap<String, i64>, Stop> {
    let rows = sqlx::query("SELECT opponent, SUM(won) FROM payouts WHERE user = ? GROUP BY opponent")
        .bind(user)
        .fetch_all(&mut *db)
        .await?;
    let mut wins = std::collections::HashMap::new();
    for row in rows {
        if let Some(opponent) = row.try_get::<Option<String>, _>(0)? {
            wins.insert(opponent, row.try_get::<Option<i64>, _>(1)?.unwrap_or(0));
        }
    }
    Ok(wins)
}

/// `CareerProgress.view`: an achievement as the player sees it, from Java's template (`achievementTemplates`). With
/// tiers, the target, pay and text are the next tier's (the top one's once all are earned); names and texts of a
/// secret one show once it's earned. Fields in `CareerAchievement`'s order, nulls left out as Gson does.
pub(crate) fn achievement(
    template: &Value,
    earned: &std::collections::HashMap<String, i64>,
    value: i64,
) -> Option<Value> {
    let id = template["id"].as_str()?;
    let secret = template["secret"].as_bool().unwrap_or(false);
    let int = |value: &Value, field: &str| value[field].as_i64().unwrap_or(0);
    let tiers = template["tiers"].as_array().map(Vec::as_slice).unwrap_or_default();
    let (achieved, at, tier, target, progress, shown, points, points_total, tier_views);
    if tiers.is_empty() {
        let earned_at = earned.get(id).copied();
        achieved = earned_at.is_some();
        at = earned_at.unwrap_or(0);
        tier = i64::from(achieved);
        target = int(template, "target");
        progress = if achieved { target } else { target.min(value) };
        shown = template;
        points_total = int(template, "points");
        points = if achieved { points_total } else { 0 };
        tier_views = Vec::new();
    } else {
        let (mut reached, mut latest, mut sum, mut total, mut next) = (0, 0, 0, 0, None);
        let mut views = Vec::new();
        for (index, step) in tiers.iter().enumerate() {
            let step_at = earned.get(&format!("{id}:{}", index + 1)).copied();
            let mut view = serde_json::Map::new();
            for field in ["target", "coins", "xp", "points"] {
                view.insert(field.into(), json!(int(step, field)));
            }
            if !step["cosmetic"].is_null() {
                view.insert("cosmetic".into(), step["cosmetic"].clone());
            }
            view.insert("tokens".into(), json!(int(step, "tokens")));
            if (step_at.is_some() || !secret) && !step["text"].is_null() {
                view.insert("text".into(), step["text"].clone());
            }
            view.insert("at".into(), json!(step_at.unwrap_or(0)));
            view.insert("holders".into(), json!(0));
            views.push(Value::Object(view));
            total += int(step, "points");
            match step_at {
                Some(step_at) => {
                    reached = index as i64 + 1;
                    sum += int(step, "points");
                    latest = latest.max(step_at);
                }
                None if next.is_none() => next = Some(step),
                None => {}
            }
        }
        shown = next.unwrap_or_else(|| tiers.last().unwrap());
        achieved = reached > 0;
        at = latest;
        tier = reached;
        target = int(shown, "target");
        progress = target.min(value);
        points = sum;
        points_total = total;
        tier_views = views;
    }
    let mut view = serde_json::Map::new();
    view.insert("id".into(), json!(id));
    if achieved || !secret {
        for field in ["name", "text"] {
            let source = if field == "text" { shown } else { template };
            if !source[field].is_null() {
                view.insert(field.into(), source[field].clone());
            }
        }
    }
    view.insert("secret".into(), json!(secret));
    view.insert("achieved".into(), json!(achieved));
    view.insert("at".into(), json!(at));
    if !template["scope"].is_null() {
        view.insert("scope".into(), template["scope"].clone());
    }
    view.insert("progress".into(), json!(progress));
    view.insert("target".into(), json!(target));
    view.insert("coins".into(), json!(int(shown, "coins")));
    view.insert("xp".into(), json!(int(shown, "xp")));
    if !shown["cosmetic"].is_null() {
        view.insert("cosmetic".into(), shown["cosmetic"].clone());
    }
    view.insert("tokens".into(), json!(0));
    if !template["family"].is_null() {
        view.insert("family".into(), template["family"].clone());
    }
    view.insert("tier".into(), json!(tier));
    view.insert("tiers".into(), Value::Array(tier_views));
    view.insert("points".into(), json!(points));
    view.insert("pointsTotal".into(), json!(points_total));
    view.insert("holders".into(), json!(0));
    Some(Value::Object(view))
}

/// the nemesis's opponent id, from solo.db's `career.nemesis` state; no solo.db, no nemesis
pub(crate) async fn nemesis(solo: &std::path::Path, user: &str) -> Result<Option<String>, Stop> {
    let Some(mut db) = crate::support::open_read_only(solo).await else { return Ok(None) };
    let value: Option<String> =
        sqlx::query_scalar("SELECT value FROM user_state WHERE user = ? AND key = 'career.nemesis'")
            .bind(user)
            .fetch_optional(&mut db)
            .await?;
    match value {
        None => Ok(None),
        Some(text) => {
            let parsed: Value = serde_json::from_str(&text).map_err(|_| Stop::Decline)?;
            Ok(parsed.get("opponentId").and_then(Value::as_str).map(str::to_owned))
        }
    }
}

/// `CareerService.opponents`: Java's roster (as a player without wins sees it, and as their nemesis) with this
/// player's wins, the tiers and rival chapters they opened, and their nemesis
pub(crate) fn opponents(
    roster: Value,
    wins: &std::collections::HashMap<String, i64>,
    nemesis: Option<&str>,
) -> Option<Value> {
    let roster = roster.as_array()?;
    let won = |id: &str| wins.get(id).copied().unwrap_or(0);
    let mut by_tier = std::collections::HashMap::<i64, i64>::new();
    for entry in roster {
        let view = &entry["view"];
        *by_tier.entry(view["tier"].as_i64()?).or_default() += won(view["id"].as_str()?);
    }
    let mut views = Vec::new();
    for entry in roster {
        let id = entry["view"]["id"].as_str()?;
        let mut view = if nemesis == Some(id) { entry["nemesis"].clone() } else { entry["view"].clone() };
        let tier = view["tier"].as_i64()?;
        let tier_open = tier == 1 || by_tier.get(&(tier - 1)).copied().unwrap_or(0) >= WINS_TO_UNLOCK_NEXT_TIER;
        let rival_open = entry.get("after").and_then(Value::as_str).is_none_or(|before| won(before) > 0);
        let object = view.as_object_mut()?;
        object.insert("wins".into(), json!(won(id)));
        object.insert("unlocked".into(), json!(tier_open && rival_open));
        views.push(view);
    }
    Some(Value::Array(views))
}

pub(crate) async fn career_user(caller: &Caller, admin: &mut SqliteConnection) -> Result<String, Stop> {
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
pub(crate) fn stored(json: Option<String>) -> Answer {
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
            "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
            "CREATE TABLE profile (user TEXT PRIMARY KEY, created INTEGER NOT NULL, starter TEXT NOT NULL, \
             coins INTEGER NOT NULL DEFAULT 0, xp INTEGER NOT NULL DEFAULT 0, wins INTEGER NOT NULL DEFAULT 0, \
             losses INTEGER NOT NULL DEFAULT 0, wc_common INTEGER NOT NULL DEFAULT 0, wc_uncommon INTEGER NOT NULL DEFAULT 0, \
             wc_rare INTEGER NOT NULL DEFAULT 0, wc_mythic INTEGER NOT NULL DEFAULT 0, pack_tokens INTEGER NOT NULL DEFAULT 0, \
             show_progress INTEGER NOT NULL DEFAULT 1, capsule_tokens INTEGER NOT NULL DEFAULT 0, quest_day TEXT, reroll_day TEXT, \
             quest_week TEXT)",
            "CREATE TABLE quests (user TEXT NOT NULL, slot INTEGER NOT NULL, quest TEXT NOT NULL, \
             progress INTEGER NOT NULL DEFAULT 0, assigned TEXT NOT NULL, PRIMARY KEY (user, slot))",
            "CREATE TABLE card_styles (user TEXT NOT NULL, card TEXT NOT NULL, item TEXT NOT NULL, PRIMARY KEY (user, card))",
            "INSERT INTO quests VALUES ('neri', 0, 'win3', 2, 'x'), ('neri', 1, 'gone', 0, 'x'), ('neri', 3, 'week', 5, 'x')",
            "CREATE TABLE unlocks (user TEXT NOT NULL, kind TEXT NOT NULL, item TEXT NOT NULL, at INTEGER NOT NULL)",
            "CREATE TABLE progress_games (game_key TEXT PRIMARY KEY, user TEXT NOT NULL, at INTEGER NOT NULL, details TEXT)",
            "CREATE TABLE payouts (match_key TEXT PRIMARY KEY, user TEXT NOT NULL, at INTEGER NOT NULL, opponent TEXT NOT NULL, \
             won INTEGER NOT NULL, coins INTEGER NOT NULL, xp INTEGER NOT NULL, note TEXT)",
            "INSERT INTO payouts VALUES ('m1', 'neri', 1, 'a', 1, 25, 10, NULL), ('m2', 'neri', 2, 'a', 0, 5, 5, NULL)",
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
        let views = dir.join("career-views.json");
        stores.add_career_views(&views);
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

        // content views: Java's file, read again when it changes; Java answers until it's written
        assert!(matches!(stores.call(&neri, &request("careerStarters", json!([])), true).await, Outcome::Declined));
        std::fs::write(&views, r#"{"starters":[{"id":"red"}],"cosmetics":[]}"#).unwrap();
        assert_eq!(call("careerStarters", json!([])).await, json!([{"id": "red"}]));
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&views, r#"{"starters":[{"id":"blue"}],"cosmetics":[{"kind":"avatar"}]}"#).unwrap();
        assert_eq!(call("careerStarters", json!([])).await, json!([{"id": "blue"}]));
        assert_eq!(call("careerCosmetics", json!([])).await, json!([{"kind": "avatar"}]));

        // the roster: Java's views with neri's wins and nemesis (solo.db, under the name as signed in)
        std::thread::sleep(std::time::Duration::from_millis(20));
        let view =
            |id: &str, nemesis: bool| json!({"id": id, "tier": 1, "unlocked": true, "wins": 0, "nemesis": nemesis});
        let roster = json!({"opponents": [
            {"view": view("a", false), "nemesis": view("a", true)},
            {"view": view("b", false), "nemesis": view("b", true)},
        ]});
        std::fs::write(&views, roster.to_string()).unwrap();
        let mut solo =
            SqliteConnectOptions::new().filename(dir.join("solo.db")).create_if_missing(true).connect().await.unwrap();
        for sql in [
            "CREATE TABLE user_state (user TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL, updated INTEGER NOT NULL, PRIMARY KEY (user, key))",
            "INSERT INTO user_state VALUES ('Neri', 'career.nemesis', '{\"opponentId\":\"b\",\"since\":1}', 1)",
        ] {
            sqlx::query(sql).execute(&mut solo).await.unwrap();
        }
        // quests: Java picks a new day's at random, Rust answers once they're picked
        let roster_file = std::fs::read_to_string(&views).unwrap();
        let mut with_quests: Value = serde_json::from_str(&roster_file).unwrap();
        with_quests["styles"] = json!({
            "m21-zap": {"card": "Zap", "kind": "cardstyle"},
            "m21-arrow": {"card": "Arrow", "kind": "cardstyle"},
            "m21-sleeve": {"kind": "sleeve"},
        });
        with_quests["quests"] = json!([
            {"id": "win3", "text": "Win 3 games", "counter": "wins", "target": 3, "coins": 20, "xp": 30, "weekly": false},
            {"id": "week", "text": "Win 10", "counter": "wins", "target": 10, "coins": 50, "xp": 90, "weekly": true,
             "counts": "Games against people"},
        ]);
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&views, with_quests.to_string()).unwrap();
        let (today, week) = utc_day_and_week(crate::admin::now_millis());
        sqlx::query("UPDATE profile SET quest_day = ?, quest_week = ?, reroll_day = ? WHERE user = 'neri'")
            .bind(&today)
            .bind(&week)
            .bind(&today)
            .execute(&mut db)
            .await
            .unwrap();
        assert_eq!(
            call("careerQuests", json!([])).await,
            json!({"quests": [
                {"slot": 0, "id": "win3", "text": "Win 3 games", "weekly": false, "progress": 2, "target": 3, "coins": 20, "xp": 30},
                {"slot": 3, "id": "week", "text": "Win 10", "counts": "Games against people", "weekly": true, "progress": 5,
                 "target": 10, "coins": 50, "xp": 90},
            ], "canReroll": false, "today": today})
        );
        // a new day: Rust picks the day's quests with the seeded dice, as Java does
        sqlx::query("UPDATE profile SET quest_day = '2000-01-01', quest_week = '2000-W01' WHERE user = 'neri'")
            .execute(&mut db)
            .await
            .unwrap();
        let fresh = call("careerQuests", json!([])).await;
        let picked: Vec<_> = fresh["quests"].as_array().unwrap().iter().map(|q| q["id"].clone()).collect();
        assert!(picked.contains(&json!("win3")) && picked.contains(&json!("week")), "{fresh}");
        // card styles: only one owned, on its own card
        sqlx::query(
            "INSERT INTO unlocks VALUES ('neri', 'cardstyle', 'm21-zap', 6), ('neri', 'cardstyle', 'm21-arrow', 7)",
        )
        .execute(&mut db)
        .await
        .unwrap();
        let refused = async |params: Value| match stores.call(&neri, &request("careerEquipStyle", params), true).await {
            Outcome::Error { message, .. } => message,
            other => panic!("{other:?}"),
        };
        assert_eq!(refused(json!(["", "m21-zap"])).await, "Which card?");
        assert_eq!(refused(json!(["Arrow", "m21-zap"])).await, "That style isn't for Arrow.");
        assert_eq!(refused(json!(["Zap", "nope"])).await, "That style isn't for Zap.");
        assert!(matches!(
            stores.call(&neri, &request("careerEquipStyle", json!(["Zap", "m21-sleeve"])), true).await,
            Outcome::Declined
        ));
        assert_eq!(call("careerEquipStyle", json!(["Zap", "m21-zap"])).await, json!({"Zap": "m21-zap"}));
        assert_eq!(
            call("careerEquipStyle", json!(["Arrow", "m21-arrow"])).await,
            json!({"Arrow": "m21-arrow", "Zap": "m21-zap"})
        );
        assert_eq!(call("careerEquipStyle", json!(["Zap", ""])).await, json!({"Arrow": "m21-arrow"}));
        sqlx::query("DELETE FROM unlocks WHERE item = 'm21-zap'").execute(&mut db).await.unwrap();
        assert_eq!(refused(json!(["Zap", "m21-zap"])).await, "Pull that style from its machine first.");
        let opponents = call("careerOpponents", json!([])).await;
        assert_eq!(
            (opponents[0]["wins"].clone(), opponents[0]["nemesis"].clone(), opponents[1]["nemesis"].clone()),
            (json!(1), json!(false), json!(true))
        );

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

    /// crafting against a real catalog export (`PLAYMAT_TEST_CATALOG`, the file `-Dxmage.catalog.export` writes)
    #[tokio::test]
    async fn craft_pays_wildcards_coins_and_achievements() {
        let Ok(catalog) = std::env::var("PLAYMAT_TEST_CATALOG") else { return };
        let dir = std::env::temp_dir().join(format!("playmat-craft-{}", uuid::Uuid::new_v4()));
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
            "CREATE TABLE profile (user TEXT PRIMARY KEY, created INTEGER NOT NULL, starter TEXT NOT NULL, \
             coins INTEGER NOT NULL DEFAULT 0, xp INTEGER NOT NULL DEFAULT 0, wins INTEGER NOT NULL DEFAULT 0, \
             losses INTEGER NOT NULL DEFAULT 0, wc_common INTEGER NOT NULL DEFAULT 0, wc_uncommon INTEGER NOT NULL DEFAULT 0, \
             wc_rare INTEGER NOT NULL DEFAULT 0, wc_mythic INTEGER NOT NULL DEFAULT 0, pack_tokens INTEGER NOT NULL DEFAULT 0, \
             show_progress INTEGER NOT NULL DEFAULT 1, capsule_tokens INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE stats (user TEXT NOT NULL, counter TEXT NOT NULL, value INTEGER NOT NULL, PRIMARY KEY (user, counter))",
            "CREATE TABLE achievements (user TEXT NOT NULL, id TEXT NOT NULL, at INTEGER NOT NULL, PRIMARY KEY (user, id))",
            "CREATE TABLE unlocks (user TEXT NOT NULL, kind TEXT NOT NULL, item TEXT NOT NULL, at INTEGER NOT NULL)",
            "CREATE TABLE payouts (match_key TEXT PRIMARY KEY, user TEXT NOT NULL, at INTEGER NOT NULL, opponent TEXT NOT NULL, \
             won INTEGER NOT NULL, coins INTEGER NOT NULL, xp INTEGER NOT NULL, note TEXT)",
            "CREATE TABLE token_log (user TEXT NOT NULL, at INTEGER NOT NULL, amount INTEGER NOT NULL, reason TEXT NOT NULL)",
            "INSERT INTO profile (user, created, starter, coins, wc_uncommon) VALUES ('neri', 1, 'red', 30, 1)",
            "INSERT INTO stats VALUES ('neri', 'cards_crafted', 8)",
        ] {
            sqlx::query(sql).execute(&mut db).await.unwrap();
        }
        let mut admin = SqliteConnectOptions::new().filename(dir.join("web_admin.db")).connect().await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS `admin_setting` (`name` VARCHAR , `value` VARCHAR , `changedBy` VARCHAR , `changedAt` BIGINT , PRIMARY KEY (`name`) )")
            .execute(&mut admin)
            .await
            .unwrap();
        stores.add_career(&dir.join("career.db"), Some(&content_dir())).await.unwrap();
        let views = dir.join("career-views.json");
        std::fs::write(
            &views,
            json!({"opponents": [], "achievements": [
                {"id": "craft-10", "scope": "total", "counter": "cards_crafted", "target": 10, "coins": 0, "xp": 100, "tiers": []},
                {"id": "t-crafted", "scope": "total", "counter": "cards_crafted", "target": 0, "tiers": [
                    {"target": 10, "coins": 25, "xp": 50, "points": 5, "tokens": 2},
                    {"target": 50, "coins": 50, "xp": 100, "points": 10, "tokens": 0},
                ]},
            ]})
            .to_string(),
        )
        .unwrap();
        stores.add_career_views(&views);
        stores.set_catalog(std::sync::Arc::new(
            crate::catalog::Catalog::open(std::path::Path::new(&catalog)).await.unwrap(),
        ));
        let neri =
            Caller { user: Some("Neri".into()), admin: false, account: true, ip: "127.0.0.1".into(), connection: 1 };
        let craft = async |set: &str, number: &str| {
            stores.call(&neri, &request("careerCraft", json!([set, number])), true).await
        };
        let refused = async |set: &str, number: &str| match craft(set, number).await {
            Outcome::Error { message, .. } => message,
            other => panic!("{other:?}"),
        };
        assert_eq!(refused("M21", "9999").await, "There is no card M21 9999.");
        assert_eq!(refused("M21", "272").await, "Basic lands are free; there's nothing to craft.");
        assert_eq!(refused("M21", "7").await, "Basri Ket takes a mythic wildcard; you have none.");
        // an uncommon spends its wildcard first, a common its coins; the tenth card crafted pays both achievements
        let Outcome::Result(_) = craft("m21", "159").await else { panic!() };
        let one: (i64, i64, i64) =
            sqlx::query_as("SELECT coins, wc_uncommon, xp FROM profile").fetch_one(&mut db).await.unwrap();
        assert_eq!(one, (5, 1, 0));
        let uncommon: (String, String) = sqlx::query_as(
            "SELECT set_code, card_number FROM printings WHERE rarity = 'UNCOMMON' AND set_code = 'M21' LIMIT 1",
        )
        .fetch_one(&mut SqliteConnectOptions::new().filename(&catalog).read_only(true).connect().await.unwrap())
        .await
        .unwrap();
        let Outcome::Result(profile) = craft(&uncommon.0, &uncommon.1).await else { panic!() };
        let two: (i64, i64, i64, i64) = sqlx::query_as("SELECT coins, wc_uncommon, xp, capsule_tokens FROM profile")
            .fetch_one(&mut db)
            .await
            .unwrap();
        assert_eq!(two, (30, 0, 150, 2));
        assert_eq!(profile["coins"], json!(30));
        let earned: Vec<String> =
            sqlx::query_scalar("SELECT id FROM achievements ORDER BY id").fetch_all(&mut db).await.unwrap();
        assert_eq!(earned, ["craft-10", "t-crafted:1"]);
        let log: (i64, String) =
            sqlx::query_as("SELECT amount, reason FROM token_log").fetch_one(&mut db).await.unwrap();
        assert_eq!(log, (2, "achievement t-crafted:1".into()));
        sqlx::query("UPDATE profile SET coins = 10").execute(&mut db).await.unwrap();
        assert_eq!(refused("m21", "159").await, "Shock costs 25 coins to craft; you have 10.");
        sqlx::query("UPDATE cards SET count = 4 WHERE name = 'Shock'").execute(&mut db).await.unwrap();
        assert_eq!(refused("m21", "159").await, "You already have 4 copies of Shock.");
    }

    #[tokio::test]
    async fn start_and_save_a_deck() {
        let dir = std::env::temp_dir().join(format!("playmat-start-{}", uuid::Uuid::new_v4()));
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
            "CREATE TABLE profile (user TEXT PRIMARY KEY, created INTEGER NOT NULL, starter TEXT NOT NULL, \
             coins INTEGER NOT NULL DEFAULT 0, xp INTEGER NOT NULL DEFAULT 0, wins INTEGER NOT NULL DEFAULT 0, \
             losses INTEGER NOT NULL DEFAULT 0, wc_common INTEGER NOT NULL DEFAULT 0, wc_uncommon INTEGER NOT NULL DEFAULT 0, \
             wc_rare INTEGER NOT NULL DEFAULT 0, wc_mythic INTEGER NOT NULL DEFAULT 0, pack_tokens INTEGER NOT NULL DEFAULT 0, \
             show_progress INTEGER NOT NULL DEFAULT 1, capsule_tokens INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE decks (user TEXT PRIMARY KEY, deck TEXT NOT NULL, updated INTEGER NOT NULL)",
            "CREATE TABLE unlocks (user TEXT NOT NULL, kind TEXT NOT NULL, item TEXT NOT NULL, at INTEGER NOT NULL)",
            // progression made a profile, without a collection
            "INSERT INTO profile (user, created, starter, coins) VALUES ('neri', 1, '', 7)",
        ] {
            sqlx::query(sql).execute(&mut db).await.unwrap();
        }
        let mut admin = SqliteConnectOptions::new().filename(dir.join("web_admin.db")).connect().await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS `admin_setting` (`name` VARCHAR , `value` VARCHAR , `changedBy` VARCHAR , `changedAt` BIGINT , PRIMARY KEY (`name`) )")
            .execute(&mut admin)
            .await
            .unwrap();
        stores.add_career(&dir.join("career.db"), Some(&content_dir())).await.unwrap();
        let views = dir.join("career-views.json");
        std::fs::write(
            &views,
            json!({
                "starters": [{"id": "red"}, {"id": "broken"}],
                "starterCards": {"red": [
                    {"name": "Shock", "setCode": "M21", "cardNumber": "159", "rarity": "common", "amount": 4},
                    {"name": "Fire Elemental", "setCode": "M21", "cardNumber": "141", "rarity": "common", "amount": 2},
                ]},
            })
            .to_string(),
        )
        .unwrap();
        stores.add_career_views(&views);
        let neri =
            Caller { user: Some("Neri".into()), admin: false, account: true, ip: "127.0.0.1".into(), connection: 1 };
        let outcome = async |method: &str, params: Value| stores.call(&neri, &request(method, params), true).await;
        let refused = async |method: &str, params: Value| match outcome(method, params).await {
            Outcome::Error { message, .. } => message,
            other => panic!("{other:?}"),
        };
        let deck = json!({"name": "Burn", "extra": 1, "cards": [
            {"cardName": "Shock", "setCode": "M21", "cardNumber": "159", "amount": 3},
            {"cardName": "Mountain", "amount": 12},
        ], "sideboard": [{"cardName": "Shock", "amount": 2}]});
        assert_eq!(refused("careerSaveDeck", json!([deck])).await, "Open your Career collection first.");
        assert_eq!(refused("careerStart", json!(["blue"])).await, "There is no starter deck blue.");
        assert!(matches!(outcome("careerStart", json!(["broken"])).await, Outcome::Declined));
        let Outcome::Result(profile) = outcome("careerStart", json!(["red"])).await else { panic!() };
        assert_eq!(
            (profile["starter"].clone(), profile["coins"].clone(), profile["cards"].clone()),
            (json!("red"), json!(107), json!(6))
        );
        assert_eq!(refused("careerStart", json!(["red"])).await, "You already have a Career.");
        // five Shocks across deck and sideboard, four owned; the Mountains are free
        assert_eq!(
            refused("careerSaveDeck", json!([deck])).await,
            "The deck has cards you don't own: Shock: 5 in the deck, 4 owned."
        );
        let mut fits = deck.clone();
        fits["sideboard"] = json!([{"cardName": "Fire Elemental", "amount": 1}]);
        let Outcome::Result(saved) = outcome("careerSaveDeck", json!([fits])).await else { panic!() };
        assert_eq!(saved, json!(true));
        let stored: String =
            sqlx::query_scalar("SELECT deck FROM decks WHERE user = 'neri'").fetch_one(&mut db).await.unwrap();
        // as Gson writes the class back: unknown fields gone, every card with its amount
        assert_eq!(
            serde_json::from_str::<Value>(&stored).unwrap(),
            json!({"name": "Burn", "cards": [
                {"cardName": "Shock", "setCode": "M21", "cardNumber": "159", "amount": 3},
                {"cardName": "Mountain", "amount": 12},
            ], "sideboard": [{"cardName": "Fire Elemental", "amount": 1}]})
        );
        assert!(matches!(outcome("careerSaveDeck", json!([{"cards": [{"amount": 1}]}])).await, Outcome::Declined));
    }

    fn content_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../Mage.Server/src/main/resources/career")
    }

    #[test]
    fn achievements_as_java_shows_them() {
        let earned: std::collections::HashMap<String, i64> =
            [("win:1".into(), 5), ("win:2".into(), 9), ("hidden".into(), 7)].into();
        // tiered: the next tier's target, pay and text; the earned tiers' points and latest time
        let tiered = json!({"id": "win", "name": "Winner", "text": "Win 1 game.", "secret": false, "scope": "total",
        "counter": "wins", "family": "play", "target": 1, "coins": 0, "xp": 0, "points": 10, "tiers": [
            {"target": 1, "coins": 5, "xp": 1, "points": 5, "tokens": 0, "text": "Win 1 game."},
            {"target": 10, "coins": 10, "xp": 2, "points": 10, "tokens": 1, "text": "Win 10 games."},
            {"target": 100, "coins": 20, "xp": 3, "points": 20, "cosmetic": {"kind": "title", "id": "champ"}, "tokens": 2,
             "text": "Win 100 games."},
        ]});
        let view = achievement(&tiered, &earned, 42).unwrap();
        assert_eq!(
            view.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(),
            [
                "id",
                "name",
                "text",
                "secret",
                "achieved",
                "at",
                "scope",
                "progress",
                "target",
                "coins",
                "xp",
                "cosmetic",
                "tokens",
                "family",
                "tier",
                "tiers",
                "points",
                "pointsTotal",
                "holders"
            ]
        );
        assert_eq!(
            (
                view["text"].clone(),
                view["tier"].clone(),
                view["at"].clone(),
                view["progress"].clone(),
                view["target"].clone()
            ),
            (json!("Win 100 games."), json!(2), json!(9), json!(42), json!(100))
        );
        assert_eq!((view["points"].clone(), view["pointsTotal"].clone()), (json!(15), json!(35)));
        assert_eq!(
            view["tiers"][1],
            json!({"target": 10, "coins": 10, "xp": 2, "points": 10, "tokens": 1,
            "text": "Win 10 games.", "at": 9, "holders": 0})
        );
        // a secret one shows its name and text once earned
        let secret = json!({"id": "hidden", "name": "Hidden", "text": "Do it.", "secret": true, "scope": "game",
            "target": 1, "coins": 3, "xp": 0, "points": 15, "tiers": []});
        let shown = achievement(&secret, &earned, 0).unwrap();
        assert_eq!(
            (shown["name"].clone(), shown["progress"].clone(), shown["points"].clone()),
            (json!("Hidden"), json!(1), json!(15))
        );
        let hidden = achievement(&secret, &Default::default(), 0).unwrap();
        assert!(hidden.get("name").is_none() && hidden.get("text").is_none());
        assert_eq!((hidden["achieved"].clone(), hidden["points"].clone()), (json!(false), json!(0)));
    }

    #[test]
    fn days_and_iso_weeks_in_utc() {
        let at = |y, m, d| crate::support::days_from_civil(y, m, d) * 86_400_000 + 3_600_000;
        assert_eq!(utc_day_and_week(at(2026, 11, 4)), ("2026-11-04".into(), "2026-W45".into()));
        // the first days of January can belong to the year before's last week, and the end of December to the next
        assert_eq!(utc_day_and_week(at(2027, 1, 1)).1, "2026-W53");
        assert_eq!(utc_day_and_week(at(2025, 12, 29)).1, "2026-W01");
        assert_eq!(utc_day_and_week(at(2024, 12, 30)).1, "2025-W01");
    }

    #[test]
    fn opponents_open_by_tier_and_rival_chapter() {
        let entry = |id: &str, tier: i64, after: Option<&str>| {
            let view = json!({"id": id, "tier": tier, "unlocked": false, "wins": 0, "nemesis": false, "bounty": 0});
            let mut nemesis = view.clone();
            nemesis["nemesis"] = json!(true);
            nemesis["bounty"] = json!(40);
            let mut entry = json!({"view": view, "nemesis": nemesis});
            if let Some(after) = after {
                entry["after"] = json!(after);
            }
            entry
        };
        let roster = json!([entry("a", 1, None), entry("b", 1, None), entry("c", 2, None), entry("d", 2, Some("c"))]);
        let wins: std::collections::HashMap<String, i64> = [("a".into(), 2), ("b".into(), 1)].into();
        let views = opponents(roster.clone(), &wins, Some("b")).unwrap();
        let open: Vec<bool> = views.as_array().unwrap().iter().map(|v| v["unlocked"].as_bool().unwrap()).collect();
        assert_eq!(open, [true, true, true, false]);
        assert_eq!((views[1]["wins"].clone(), views[1]["bounty"].clone()), (json!(1), json!(40)));
        assert_eq!(views[0]["bounty"], json!(0));
        // two wins in tier 1 aren't enough for tier 2; a rival chapter opens when the one before is beaten
        let fewer: std::collections::HashMap<String, i64> = [("a".into(), 2), ("c".into(), 1)].into();
        let views = opponents(roster, &fewer, None).unwrap();
        let open: Vec<bool> = views.as_array().unwrap().iter().map(|v| v["unlocked"].as_bool().unwrap()).collect();
        assert_eq!(open, [true, true, false, false]);
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
