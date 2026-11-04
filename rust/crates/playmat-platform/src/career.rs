//! The Career's records that need nothing but db/career.db (Java's `CareerStore`): the collection, the saved deck and
//! what a game paid. The rest of the Career (its content, packs, quests, payouts and the games it starts) stays with
//! Java for now (docs/rust/Career.md).

use crate::admin::{opt_string, AdminStores, Answer, Stop};
use playmat_protocol::handlers::Caller;
use playmat_protocol::jsonrpc::{Request, NOT_AUTHORIZED};
use serde_json::{json, Value};
use sqlx::sqlite::SqliteConnection;
use sqlx::Row;

pub const CAREER_METHODS: [&str; 10] = [
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
];

/// `CareerRules.WINS_TO_UNLOCK_NEXT_TIER`
const WINS_TO_UNLOCK_NEXT_TIER: i64 = 3;

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
    fn view(&self, name: &str) -> Option<Value> {
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
                // `CareerProgress.totals`: the stats, the level, opponents beaten and the highest tier open
                let mut totals = std::collections::HashMap::new();
                for row in sqlx::query("SELECT counter, value FROM stats WHERE user = ?")
                    .bind(&user)
                    .fetch_all(&mut *db)
                    .await?
                {
                    totals.insert(row.try_get::<String, _>(0)?, row.try_get::<i64, _>(1)?);
                }
                let level = profile(db, &user, content).await?.and_then(|view| view["level"].as_i64()).unwrap_or(1);
                totals.insert("level".into(), level);
                let wins = wins_by_opponent(db, &user).await?;
                let beaten = roster.as_array().ok_or(Stop::Decline)?.iter().filter(|entry| {
                    entry["view"]["id"].as_str().and_then(|id| wins.get(id)).is_some_and(|won| *won > 0)
                });
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
                let mut views = Vec::new();
                for template in templates.as_array().ok_or(Stop::Decline)? {
                    let total = template["scope"] == json!("total");
                    let counter = template["counter"].as_str().and_then(|counter| totals.get(counter));
                    let progress = if total { counter.copied().unwrap_or(0) } else { 0 };
                    views.push(achievement(template, &earned, progress).ok_or(Stop::Decline)?);
                }
                Ok(Value::Array(views))
            }
            "careerQuests" => {
                let Some(templates) = self.career_views.as_ref().and_then(|views| views.view("quests")) else {
                    return Err(Stop::Decline);
                };
                let row = sqlx::query("SELECT quest_day, quest_week, reroll_day FROM profile WHERE user = ?")
                    .bind(&user)
                    .fetch_optional(&mut *db)
                    .await?;
                let Some(row) = row else { return Ok(Value::Null) };
                let (today, week) = utc_day_and_week(crate::admin::now_millis());
                let (quest_day, quest_week, reroll_day): (Option<String>, Option<String>, Option<String>) =
                    (row.try_get(0)?, row.try_get(1)?, row.try_get(2)?);
                // a new day or week picks quests at random: Java does that, Rust answers once they're picked
                if quest_day.as_deref() != Some(today.as_str()) || quest_week.as_deref() != Some(week.as_str()) {
                    return Err(Stop::Decline);
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
/// `CareerQuest`, in its fields' order, nulls left out
fn quest(template: &Value, slot: i64, progress: i64) -> Value {
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
fn utc_day_and_week(millis: i64) -> (String, String) {
    let days = millis.div_euclid(86_400_000);
    let (year, month, day) = crate::support::civil(days);
    // ISO weeks start on Monday and belong to the year of their Thursday
    let thursday = days - (days + 3).rem_euclid(7) + 3;
    let (iso_year, _, _) = crate::support::civil(thursday);
    let week = (thursday - crate::support::days_from_civil(iso_year, 1, 1)) / 7 + 1;
    (format!("{year:04}-{month:02}-{day:02}"), format!("{iso_year}-W{week:02}"))
}

/// `CareerStore.winsByOpponent`
async fn wins_by_opponent(
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
fn achievement(template: &Value, earned: &std::collections::HashMap<String, i64>, value: i64) -> Option<Value> {
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
async fn nemesis(solo: &std::path::Path, user: &str) -> Result<Option<String>, Stop> {
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
fn opponents(roster: Value, wins: &std::collections::HashMap<String, i64>, nemesis: Option<&str>) -> Option<Value> {
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
             show_progress INTEGER NOT NULL DEFAULT 1, capsule_tokens INTEGER NOT NULL DEFAULT 0, quest_day TEXT, reroll_day TEXT, \
             quest_week TEXT)",
            "CREATE TABLE quests (user TEXT NOT NULL, slot INTEGER NOT NULL, quest TEXT NOT NULL, \
             progress INTEGER NOT NULL DEFAULT 0, assigned TEXT NOT NULL, PRIMARY KEY (user, slot))",
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
        with_quests["quests"] = json!([
            {"id": "win3", "text": "Win 3 games", "counter": "wins", "target": 3, "coins": 20, "xp": 30, "weekly": false},
            {"id": "week", "text": "Win 10", "counter": "wins", "target": 10, "coins": 50, "xp": 90, "weekly": true,
             "counts": "Games against people"},
        ]);
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&views, with_quests.to_string()).unwrap();
        assert!(matches!(stores.call(&neri, &request("careerQuests", json!([])), true).await, Outcome::Declined));
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
