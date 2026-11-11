//! Career payouts in Rust (docs/rust/Payouts.md): what a finished game pays, from the `matchEnded` / `gameEnded`
//! event Java keeps in career.db's `payout_events`. Each step is Java's (`CareerService.payMatch` / `payGame`,
//! `CareerProgress.apply`, `CareerLadder.finished`), in Java's order, on the same rows, so a payout is the same row
//! for row and its `CareerGameResult` the same JSON.
//!
//! Rust pays roster duels and every game outside Career tables; the other modes stay with Java. On the `rust` route
//! the gateway pays pending events; on `shadow` it replays each event Java paid on a copy of the rows Java kept from
//! before, and compares the rows after and the result.

use crate::admin::Stop;
use crate::career::{self, CareerContent};
use crate::dice;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sqlx::sqlite::SqliteConnection;
use sqlx::Row;
use std::collections::{BTreeMap, HashMap, HashSet};

/// `CareerPayoutEvent`, as Gson writes it
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub event: String,
    pub key: String,
    pub at: i64,
    pub day: String,
    #[serde(default = "hundred")]
    pub xp_percent: i64,
    #[serde(default = "hundred")]
    pub coin_percent: i64,
    pub career: Option<CareerTable>,
    pub finished: Option<Finished>,
    #[serde(default)]
    pub won: bool,
    #[serde(default)]
    pub turns: i64,
    #[serde(default)]
    pub counters: BTreeMap<String, i64>,
    #[serde(default)]
    pub moments: Vec<Moment>,
}

fn hundred() -> i64 {
    100
}

#[derive(Debug, Clone, Deserialize)]
pub struct CareerTable {
    pub user: String,
    pub kind: String,
    pub opponent: Option<String>,
    #[serde(default)]
    pub tier: i64,
    pub assisted: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finished {
    pub kind: String,
    #[serde(default)]
    pub players: i64,
    #[serde(default)]
    pub turns: i64,
    #[serde(default)]
    pub ai_skill: i64,
    #[serde(default)]
    pub bo3: bool,
    #[serde(default)]
    pub seats: Vec<Seat>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Seat {
    pub user: String,
    #[serde(default)]
    pub won: bool,
    #[serde(default)]
    pub match_won: bool,
    #[serde(default)]
    pub counted: bool,
    #[serde(default)]
    pub counters: BTreeMap<String, i64>,
    #[serde(default)]
    pub moments: Vec<Moment>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Moment {
    pub id: String,
    #[serde(default)]
    pub value: i64,
    #[serde(default)]
    pub turn: i64,
}

impl Event {
    /// `CareerPayouts.rustPays`: roster duels and every game outside Career tables, for ASCII names
    pub fn rust_pays(&self) -> bool {
        let known = self.event == "gameEnded" || self.career.as_ref().is_some_and(|c| c.kind == "duel");
        known && self.users().iter().all(|user| user.is_ascii())
    }

    pub fn users(&self) -> Vec<String> {
        let mut users: Vec<String> = self.career.iter().map(|c| c.user.clone()).collect();
        users.extend(self.finished.iter().flat_map(|f| f.seats.iter().map(|s| s.user.clone())));
        users
    }
}

/// `CareerProgress.QUEST_SLOTS`, `WEEKLY_SLOT`, `WEEKLY_GOALS`, `WEEKLY_GOAL_XP`
const QUEST_SLOTS: i64 = 3;
const WEEKLY_SLOT: i64 = 3;
const WEEKLY_GOALS: [i64; 3] = [5, 10, 15];
const WEEKLY_GOAL_XP: i64 = 250;
/// `CareerRules.MIN_TURN_FOR_REWARD`
const MIN_TURN_FOR_REWARD: i64 = 3;
/// `CareerLadder.NEMESIS_LIFE`, `NEMESIS_BOUNTY`
const NEMESIS_LIFE: i64 = 3;
const NEMESIS_BOUNTY: i64 = 40;
/// `CareerProgress.MAX_COUNTERS`
const MAX_COUNTERS: [&str; 7] =
    ["biggest_hit", "max_attacker_power", "most_spells_in_a_turn", "most_mana_in_a_turn", "turns", "life_at_end", "my_turns"];
/// `CareerService.FORMATS`
const FORMATS: [&str; 9] = ["standard", "pioneer", "modern", "legacy", "vintage", "pauper", "commander", "brawl", "limited"];

/// The nemesis, Java's solo.db (`career.nemesis`, by the name as signed in), or the copy shadow replays on
pub enum Nemeses {
    Solo(std::path::PathBuf),
    Copy(HashMap<String, Option<String>>),
}

impl Nemeses {
    async fn get(&self, user: &str) -> Result<Option<String>, Stop> {
        let value = match self {
            Nemeses::Copy(map) => map.get(user).cloned().flatten(),
            Nemeses::Solo(path) => {
                let Some(mut db) = crate::support::open_read_only(path).await else { return Ok(None) };
                sqlx::query_scalar("SELECT value FROM user_state WHERE user = ? AND key = 'career.nemesis'")
                    .bind(user)
                    .fetch_optional(&mut db)
                    .await?
            }
        };
        // Gson reads "null" as no nemesis
        Ok(value.and_then(|text| serde_json::from_str::<Value>(&text).ok()).filter(|v| v.is_object()).and_then(
            |v| v.get("opponentId").and_then(Value::as_str).map(str::to_owned),
        ))
    }

    async fn put(&mut self, user: &str, value: &str, now: i64) -> Result<(), Stop> {
        match self {
            Nemeses::Copy(map) => {
                map.insert(user.to_owned(), Some(value.to_owned()));
            }
            Nemeses::Solo(path) => {
                use sqlx::ConnectOptions;
                let mut db = sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(path)
                    .busy_timeout(std::time::Duration::from_secs(10))
                    .connect()
                    .await?;
                sqlx::query(
                    "INSERT INTO user_state (user, key, value, updated) VALUES (?, 'career.nemesis', ?, ?) \
                     ON CONFLICT (user, key) DO UPDATE SET value = excluded.value, updated = excluded.updated",
                )
                .bind(user)
                .bind(value)
                .bind(now)
                .execute(&mut db)
                .await?;
            }
        }
        Ok(())
    }
}

/// What a payout reads besides career.db: the content and the views Java writes
pub struct Book<'a> {
    pub content: &'a CareerContent,
    pub quests: &'a Value,
    pub achievements: &'a Value,
    pub roster: &'a Value,
    pub cosmetics: &'a Value,
    /// career.db's secret, for the dice
    pub secret: &'a str,
}

/// One payout in progress: the event's day and settings, the rows, the nemesis
struct Pay<'a, 'b> {
    db: &'b mut SqliteConnection,
    book: &'b Book<'a>,
    nemeses: &'b mut Nemeses,
    day: String,
    week: String,
    xp_percent: i64,
    coin_percent: i64,
}

/// `CareerStore.key`
fn key(user: &str) -> String {
    user.to_lowercase()
}

fn int(value: &Value, field: &str) -> i64 {
    value.get(field).and_then(Value::as_i64).unwrap_or(0)
}

/// a `CareerBonus`, as Gson writes it
fn bonus(text: &str, xp: i64, coins: i64, tokens: i64) -> Value {
    json!({"text": text, "xp": xp, "coins": coins, "tokens": tokens})
}

/// A `CareerGameResult` being filled, in its fields' order; `finish` writes it as Gson does
#[derive(Default)]
struct Result_ {
    game_key: String,
    kind: String,
    xp_before: i64,
    bonuses: Vec<Value>,
    career: bool,
    won: bool,
    opponent: Option<String>,
    coins: i64,
    xp: i64,
    note: Option<String>,
    level_before: i64,
    level_after: i64,
    xp_into_level: i64,
    xp_for_next: i64,
    level_rewards: Vec<Value>,
    quests: Vec<Value>,
    masteries: Vec<Value>,
    renown_before: i64,
    renown_after: i64,
    achievements: Vec<Value>,
    moments: Vec<Value>,
    weekly: Option<Value>,
    packs: i64,
    tokens: i64,
    opened: Vec<String>,
    mode: Option<Mode>,
    assisted: Option<String>,
}

/// `CareerModeResult`
struct Mode {
    kind: String,
    title: String,
    lines: Vec<String>,
    coins: i64,
    xp: i64,
    reference: Option<String>,
}

impl Result_ {
    fn json(&self) -> Value {
        let mut out = Map::new();
        out.insert("gameKey".into(), json!(self.game_key));
        out.insert("kind".into(), json!(self.kind));
        out.insert("xpBefore".into(), json!(self.xp_before));
        out.insert("bonuses".into(), json!(self.bonuses));
        out.insert("career".into(), json!(self.career));
        out.insert("won".into(), json!(self.won));
        if let Some(opponent) = &self.opponent {
            out.insert("opponent".into(), json!(opponent));
        }
        out.insert("coins".into(), json!(self.coins));
        out.insert("xp".into(), json!(self.xp));
        if let Some(note) = &self.note {
            out.insert("note".into(), json!(note));
        }
        for (field, value) in [
            ("levelBefore", self.level_before),
            ("levelAfter", self.level_after),
            ("xpIntoLevel", self.xp_into_level),
            ("xpForNext", self.xp_for_next),
        ] {
            out.insert(field.into(), json!(value));
        }
        out.insert("levelRewards".into(), json!(self.level_rewards));
        out.insert("quests".into(), json!(self.quests));
        out.insert("masteries".into(), json!(self.masteries));
        out.insert("renownBefore".into(), json!(self.renown_before));
        out.insert("renownAfter".into(), json!(self.renown_after));
        out.insert("achievements".into(), json!(self.achievements));
        out.insert("moments".into(), json!(self.moments));
        if let Some(weekly) = &self.weekly {
            out.insert("weekly".into(), weekly.clone());
        }
        out.insert("packs".into(), json!(self.packs));
        out.insert("tokens".into(), json!(self.tokens));
        out.insert("opened".into(), json!(self.opened));
        if let Some(mode) = &self.mode {
            let mut view = Map::new();
            view.insert("kind".into(), json!(mode.kind));
            view.insert("title".into(), json!(mode.title));
            view.insert("lines".into(), json!(mode.lines));
            view.insert("coins".into(), json!(mode.coins));
            view.insert("xp".into(), json!(mode.xp));
            view.insert("stars".into(), json!(0));
            if let Some(reference) = &mode.reference {
                view.insert("ref".into(), json!(reference));
            }
            view.insert("trial".into(), json!(false));
            view.insert("cosmetics".into(), json!([]));
            out.insert("mode".into(), Value::Object(view));
        }
        if let Some(assisted) = &self.assisted {
            out.insert("assisted".into(), json!(assisted));
        }
        Value::Object(out)
    }
}

/// Pays one event, inside the caller's transaction on `db`.
///
/// Ok(true) when it was paid, Ok(false) when it paid nothing (paid before, no profile); Err(Decline) for an event
/// Rust doesn't pay.
pub(crate) async fn pay(
    db: &mut SqliteConnection,
    book: &Book<'_>,
    nemeses: &mut Nemeses,
    event: &Event,
) -> Result<bool, Stop> {
    if !event.rust_pays() {
        return Err(Stop::Decline);
    }
    let millis = days_from_iso(&event.day).ok_or(Stop::Decline)? * 86_400_000;
    let (day, week) = career::utc_day_and_week(millis);
    let mut pay = Pay {
        db,
        book,
        nemeses,
        day,
        week,
        xp_percent: event.xp_percent,
        coin_percent: event.coin_percent,
    };
    if event.event == "matchEnded" {
        pay.match_ended(event).await
    } else {
        pay.game_ended(event).await
    }
}

/// "2026-11-11" as days since 1970-01-01
fn days_from_iso(day: &str) -> Option<i64> {
    let mut parts = day.split('-');
    let (y, m, d) = (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
    Some(crate::support::days_from_civil(y, m, d))
}

impl Pay<'_, '_> {
    fn rewards(&self) -> &Value {
        &self.book.content.rewards
    }

    /// `CareerRewards.scaleXp`, `scaleCoins`
    fn scale_xp(&self, xp: i64) -> i64 {
        (xp * self.xp_percent) / 100
    }

    fn scale_coins(&self, coins: i64) -> i64 {
        (coins * self.coin_percent) / 100
    }

    fn turn_floor(&self) -> i64 {
        self.rewards().get("turnFloor").and_then(Value::as_i64).unwrap_or(MIN_TURN_FOR_REWARD)
    }

    fn capsule(&self, field: &str, default: i64) -> i64 {
        self.rewards().get("capsules").and_then(|c| c.get(field)).and_then(Value::as_i64).unwrap_or(default)
    }

    // ---- the store, as CareerStore does it

    async fn exec(&mut self, sql: &str, binds: &[Bind<'_>]) -> Result<u64, Stop> {
        let mut query = sqlx::query(sql);
        for bind in binds {
            query = match bind {
                Bind::Int(value) => query.bind(*value),
                Bind::Text(text) => query.bind(*text),
                Bind::Null => query.bind(Option::<String>::None),
            };
        }
        Ok(query.execute(&mut *self.db).await?.rows_affected())
    }

    async fn add_profile_int(&mut self, user: &str, column: &str, amount: i64) -> Result<(), Stop> {
        let sql = format!("UPDATE profile SET {column} = {column} + ? WHERE user = ?");
        self.exec(&sql, &[Bind::Int(amount), Bind::Text(&key(user))]).await?;
        Ok(())
    }

    async fn add_coins(&mut self, user: &str, coins: i64) -> Result<(), Stop> {
        self.add_profile_int(user, "coins", coins).await
    }

    async fn add_stat(&mut self, user: &str, counter: &str, amount: i64) -> Result<(), Stop> {
        career::add_stat(self.db, &key(user), counter, amount).await
    }

    async fn max_stat(&mut self, user: &str, counter: &str, value: i64) -> Result<(), Stop> {
        self.exec(
            "INSERT INTO stats (user, counter, value) VALUES (?, ?, ?) \
             ON CONFLICT (user, counter) DO UPDATE SET value = MAX(value, excluded.value)",
            &[Bind::Text(&key(user)), Bind::Text(counter), Bind::Int(value)],
        )
        .await?;
        Ok(())
    }

    async fn stats(&mut self, user: &str) -> Result<HashMap<String, i64>, Stop> {
        let mut stats = HashMap::new();
        for row in
            sqlx::query("SELECT counter, value FROM stats WHERE user = ?").bind(key(user)).fetch_all(&mut *self.db).await?
        {
            stats.insert(row.try_get::<String, _>(0)?, row.try_get::<i64, _>(1)?);
        }
        Ok(stats)
    }

    async fn add_unlock(&mut self, user: &str, kind: &str, item: &str, now: i64) -> Result<(), Stop> {
        self.exec(
            "INSERT OR IGNORE INTO unlocks (user, kind, item, at) VALUES (?, ?, ?, ?)",
            &[Bind::Text(&key(user)), Bind::Text(kind), Bind::Text(item), Bind::Int(now)],
        )
        .await?;
        Ok(())
    }

    /// `CareerCapsules.addTokens`
    async fn add_tokens(&mut self, user: &str, amount: i64, reason: &str, now: i64) -> Result<(), Stop> {
        if amount == 0 {
            return Ok(());
        }
        self.add_profile_int(user, "capsule_tokens", amount).await?;
        self.exec(
            "INSERT INTO token_log (user, at, amount, reason) VALUES (?, ?, ?, ?)",
            &[Bind::Text(&key(user)), Bind::Int(now), Bind::Int(amount), Bind::Text(reason)],
        )
        .await?;
        Ok(())
    }

    /// `CareerStore.profile`'s xp and level; None without a profile
    async fn xp(&mut self, user: &str) -> Result<Option<i64>, Stop> {
        Ok(sqlx::query_scalar("SELECT xp FROM profile WHERE user = ?")
            .bind(key(user))
            .fetch_optional(&mut *self.db)
            .await?)
    }

    /// `CareerService.ensureProfile`
    async fn ensure_profile(&mut self, user: &str, now: i64) -> Result<(), Stop> {
        self.exec(
            "INSERT OR IGNORE INTO profile (user, created, starter, coins) VALUES (?, ?, '', 0)",
            &[Bind::Text(&key(user)), Bind::Int(now)],
        )
        .await?;
        Ok(())
    }

    async fn claim_game(&mut self, game_key: &str, user: &str, now: i64) -> Result<bool, Stop> {
        Ok(self
            .exec(
                "INSERT OR IGNORE INTO progress_games (game_key, user, at) VALUES (?, ?, ?)",
                &[Bind::Text(game_key), Bind::Text(&key(user)), Bind::Int(now)],
            )
            .await?
            == 1)
    }

    async fn set_game(&mut self, result: &Result_, reason: &str) -> Result<(), Stop> {
        let details = result.json().to_string();
        self.exec(
            "UPDATE progress_games SET details = ? WHERE game_key = ?",
            &[Bind::Text(&details), Bind::Text(&result.game_key)],
        )
        .await?;
        self.exec(
            "UPDATE progress_games SET kind = ?, won = ?, coins = ?, xp = ?, reason = ? WHERE game_key = ?",
            &[
                Bind::Text(&result.kind),
                Bind::Int(i64::from(result.won)),
                Bind::Int(result.coins),
                Bind::Int(result.xp),
                Bind::Text(reason),
                Bind::Text(&result.game_key),
            ],
        )
        .await?;
        Ok(())
    }

    async fn quest_rows(&mut self, user: &str) -> Result<Vec<(i64, String, i64, String)>, Stop> {
        let mut rows = Vec::new();
        for row in sqlx::query("SELECT slot, quest, progress, assigned FROM quests WHERE user = ? ORDER BY slot")
            .bind(key(user))
            .fetch_all(&mut *self.db)
            .await?
        {
            rows.push((row.try_get(0)?, row.try_get(1)?, row.try_get(2)?, row.try_get(3)?));
        }
        Ok(rows)
    }

    async fn put_quest(&mut self, user: &str, slot: i64, quest: &str, progress: i64, day: &str) -> Result<(), Stop> {
        self.exec(
            "INSERT OR REPLACE INTO quests (user, slot, quest, progress, assigned) VALUES (?, ?, ?, ?, ?)",
            &[Bind::Text(&key(user)), Bind::Int(slot), Bind::Text(quest), Bind::Int(progress), Bind::Text(day)],
        )
        .await?;
        Ok(())
    }

    // ---- quests (CareerProgress.refreshQuests)

    async fn refresh_quests(&mut self, user: &str) -> Result<(), Stop> {
        let day = self.day.clone();
        refresh_quests(self.db, self.book, user, &day, &self.week.clone()).await
    }

    // ---- the day's extras (CareerService.dailyBonuses)

    async fn daily_bonuses(&mut self, user: &str, kind: &str, won: bool, result: &mut Result_) -> Result<(), Stop> {
        let row = sqlx::query("SELECT wins, games, person_games FROM daily WHERE user = ? AND day = ?")
            .bind(key(user))
            .bind(&self.day)
            .fetch_optional(&mut *self.db)
            .await?;
        let today: [i64; 3] = match row {
            Some(row) => [row.try_get(0)?, row.try_get(1)?, row.try_get(2)?],
            None => [0, 0, 0],
        };
        let person = matches!(kind, "person" | "pod" | "event");
        let ladder: Vec<i64> = self
            .rewards()
            .get("dailyWinCoins")
            .and_then(Value::as_array)
            .map(|l| l.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_default();
        if won {
            let nth = today[0] + 1;
            let coins = if nth < 1 || nth as usize > ladder.len() { 0 } else { self.scale_coins(ladder[nth as usize - 1]) };
            if coins > 0 {
                self.add_coins(user, coins).await?;
                result.coins += coins;
                result.bonuses.push(bonus(&format!("Daily win {nth} of {}", ladder.len()), 0, coins, 0));
            }
            let first_win = self.scale_xp(int(self.rewards(), "firstWinXp"));
            if today[0] == 0 && first_win > 0 {
                self.add_profile_int(user, "xp", first_win).await?;
                result.xp += first_win;
                let tokens = self.capsule("firstWinTokens", 1);
                if tokens > 0 {
                    let at = crate::admin::now_millis();
                    self.add_tokens(user, tokens, "first win of the day", at).await?;
                    result.tokens += tokens;
                }
                result.bonuses.push(bonus("First win of the day", first_win, 0, tokens));
            }
        }
        let first_person = self.scale_xp(int(self.rewards(), "firstPersonGameXp"));
        if person && today[2] == 0 && first_person > 0 {
            self.add_profile_int(user, "xp", first_person).await?;
            result.xp += first_person;
            result.bonuses.push(bonus("First game against a person today", first_person, 0, 0));
        }
        self.exec(
            "INSERT INTO daily (user, day, wins, games, person_games) VALUES (?, ?, ?, 1, ?) \
             ON CONFLICT (user, day) DO UPDATE SET wins = wins + excluded.wins, games = games + 1, \
             person_games = person_games + excluded.person_games",
            &[Bind::Text(&key(user)), Bind::Text(&self.day.clone()), Bind::Int(i64::from(won)), Bind::Int(i64::from(person))],
        )
        .await?;
        Ok(())
    }

    // ---- applying a game (CareerProgress.apply)

    async fn apply(
        &mut self,
        user: &str,
        tally: &BTreeMap<String, i64>,
        moments: &[Moment],
        result: &mut Result_,
        xp_before: i64,
        now: i64,
    ) -> Result<(), Stop> {
        self.refresh_quests(user).await?;
        // a LinkedHashMap from the tally's TreeMap: days_played is added at the end
        let mut counters: Vec<(String, i64)> = tally.iter().map(|(k, v)| (k.clone(), *v)).collect();
        let today: i64 = self.day.replace('-', "").parse().map_err(|_| Stop::Decline)?;
        let stats = self.stats(user).await?;
        if stats.get("last_day").copied().unwrap_or(0) < today {
            self.max_stat(user, "last_day", today).await?;
            match counters.iter_mut().find(|(k, _)| k == "days_played") {
                Some((_, v)) => *v += 1,
                None => counters.push(("days_played".into(), 1)),
            }
        }
        for moment in moments {
            result.moments.push(moment_view(moment));
        }
        for (counter, value) in &counters {
            if MAX_COUNTERS.contains(&counter.as_str()) {
                self.max_stat(user, counter, *value).await?;
            } else {
                self.add_stat(user, counter, *value).await?;
            }
        }
        let counter = |name: &str| counters.iter().find(|(k, _)| k == name).map(|(_, v)| *v).unwrap_or(0);

        // quests
        for (slot, quest, progress, assigned) in self.quest_rows(user).await? {
            let template = self.book.quests.as_array().into_iter().flatten().find(|t| t["id"] == json!(quest));
            let gained = template.and_then(|t| t["counter"].as_str()).map(counter).unwrap_or(0);
            let Some(template) = template.filter(|_| gained > 0) else { continue };
            let target = int(template, "target");
            let after = target.min(progress + gained);
            let mut view = Map::new();
            view.insert("id".into(), template["id"].clone());
            if !template["text"].is_null() {
                view.insert("text".into(), template["text"].clone());
            }
            view.insert("before".into(), json!(progress));
            view.insert("after".into(), json!(after));
            view.insert("target".into(), json!(target));
            let (mut completed, mut coins, mut xp) = (false, 0, 0);
            if after >= target {
                completed = true;
                coins = int(template, "coins");
                xp = int(template, "xp");
                self.exec("DELETE FROM quests WHERE user = ? AND slot = ?", &[Bind::Text(&key(user)), Bind::Int(slot)])
                    .await?;
                self.add_coins(user, coins).await?;
                self.add_profile_int(user, "xp", xp).await?;
                self.add_stat(user, "quests_completed", 1).await?;
                result.coins += coins;
                result.xp += xp;
            } else {
                self.put_quest(user, slot, &quest, after, &assigned).await?;
            }
            view.insert("completed".into(), json!(completed));
            view.insert("coins".into(), json!(coins));
            view.insert("xp".into(), json!(xp));
            result.quests.push(Value::Object(view));
        }

        // the weekly goal: every win (B287), but not puzzles, trials or the weekly challenge
        let weekly_game = result.mode.as_ref().is_none_or(|mode| !(mode.kind == "puzzle" || mode.kind == "challenge"));
        let week = self.week.clone();
        if result.won && weekly_game {
            let before: i64 = sqlx::query_scalar("SELECT wins FROM weekly WHERE user = ? AND week = ?")
                .bind(key(user))
                .bind(&week)
                .fetch_optional(&mut *self.db)
                .await?
                .unwrap_or(0);
            self.exec(
                "INSERT INTO weekly (user, week, wins) VALUES (?, ?, 1) ON CONFLICT (user, week) DO UPDATE SET wins = wins + 1",
                &[Bind::Text(&key(user)), Bind::Text(&week)],
            )
            .await?;
            let after = before + 1;
            if WEEKLY_GOALS.contains(&after) {
                self.add_profile_int(user, "pack_tokens", 1).await?;
                result.packs += 1;
                let xp = self.scale_xp(WEEKLY_GOAL_XP);
                self.add_profile_int(user, "xp", xp).await?;
                result.xp += xp;
                let tokens = self.capsule("weeklyGoalTokens", 3);
                if tokens > 0 {
                    self.add_tokens(user, tokens, "weekly goal", crate::admin::now_millis()).await?;
                    result.tokens += tokens;
                }
                result.bonuses.push(bonus(&format!("Weekly goal: {after} wins"), xp, 0, tokens));
            }
            self.max_stat(user, "weekly_best", after).await?;
        }
        let wins: i64 = sqlx::query_scalar("SELECT wins FROM weekly WHERE user = ? AND week = ?")
            .bind(key(user))
            .bind(&week)
            .fetch_optional(&mut *self.db)
            .await?
            .unwrap_or(0);
        let next = WEEKLY_GOALS.iter().copied().find(|goal| *goal > wins).unwrap_or(0);
        result.weekly = Some(json!({"week": week, "wins": wins, "goals": WEEKLY_GOALS, "nextGoal": next}));

        self.masteries_in(user, result, now).await?;
        let game: HashMap<String, i64> = counters.into_iter().collect();
        self.achievements_in(user, &game, result, now).await?;
        self.levels_in(user, xp_before, result, now).await?;
        Ok(())
    }

    /// `CareerProgress.masteriesIn`
    async fn masteries_in(&mut self, user: &str, result: &mut Result_, now: i64) -> Result<(), Stop> {
        let ranks: Vec<i64> = self
            .rewards()
            .get("masteryRankGames")
            .and_then(Value::as_array)
            .map(|l| l.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_default();
        let top = ranks.len() as i64;
        let (rank_xp, rank_coins) = (int(self.rewards(), "masteryRankXp"), int(self.rewards(), "masteryRankCoins"));
        let stats = self.stats(user).await?;
        let mut tracks: Vec<String> =
            ["W", "U", "B", "R", "G", "multicolor", "colorless"].iter().map(|c| format!("color_{c}")).collect();
        tracks.extend(FORMATS.iter().map(|f| format!("format_{f}")));
        for track in tracks {
            let games = stats.get(&format!("games_{track}")).copied().unwrap_or(0);
            let rank = ranks.iter().take_while(|needed| games >= **needed).count() as i64;
            let paid = stats.get(&format!("mastery_{track}")).copied().unwrap_or(0);
            for next in paid + 1..=rank {
                let xp = self.scale_xp(rank_xp * next);
                let coins = self.scale_coins(rank_coins * next);
                self.add_profile_int(user, "xp", xp).await?;
                self.add_coins(user, coins).await?;
                let badge = format!("mastery-{}-{next}", track.replace('_', "-")).to_lowercase();
                self.add_unlock(user, "badge", &badge, now).await?;
                let cosmetic = self.mastery_cosmetic(&track, next, top);
                if let Some(cosmetic) = &cosmetic {
                    let (kind, id) = (cosmetic["kind"].as_str().unwrap_or(""), cosmetic["id"].as_str().unwrap_or(""));
                    self.add_unlock(user, kind, id, now).await?;
                }
                result.xp += xp;
                result.coins += coins;
                let mut reached = Map::new();
                reached.insert("track".into(), json!(track));
                reached.insert("rank".into(), json!(next));
                reached.insert("xp".into(), json!(xp));
                reached.insert("coins".into(), json!(coins));
                if let Some(cosmetic) = cosmetic {
                    reached.insert("cosmetic".into(), cosmetic);
                }
                result.masteries.push(Value::Object(reached));
            }
            if rank > paid {
                self.max_stat(user, &format!("mastery_{track}"), rank).await?;
                self.add_stat(user, "mastery_ranks", rank - paid).await?;
            }
        }
        Ok(())
    }

    /// `CareerProgress.masteryCosmetic`: the catalog's, or at the top the track's master title
    fn mastery_cosmetic(&self, track: &str, rank: i64, top: i64) -> Option<Value> {
        let found = self.book.cosmetics.as_array().into_iter().flatten().find(|item| {
            item["mastery"].as_str() == Some(track) && item["rank"].as_i64().unwrap_or(0) == rank
        });
        if let Some(item) = found {
            return Some(json!({"kind": item["kind"], "id": item["id"]}));
        }
        (rank == top).then(|| {
            let suffix = &track[track.find('_').map_or(0, |i| i + 1)..];
            json!({"kind": "title", "id": format!("{}-master", suffix.to_lowercase())})
        })
    }

    /// `CareerProgress.achievementsIn` with a game's counters and the result to fill
    async fn achievements_in(
        &mut self,
        user: &str,
        game: &HashMap<String, i64>,
        result: &mut Result_,
        now: i64,
    ) -> Result<(), Stop> {
        let mut earned = HashMap::new();
        for row in sqlx::query("SELECT id, at FROM achievements WHERE user = ? ORDER BY at")
            .bind(key(user))
            .fetch_all(&mut *self.db)
            .await?
        {
            earned.insert(row.try_get::<String, _>(0)?, row.try_get::<i64, _>(1)?);
        }
        let totals = career::totals(self.db, &key(user), self.book.content, self.book.roster.clone()).await?;
        for template in self.book.achievements.as_array().ok_or(Stop::Decline)? {
            let id = template["id"].as_str().ok_or(Stop::Decline)?;
            let counter = template["counter"].as_str().unwrap_or_default();
            let value = if template["scope"] == json!("game") {
                game.get(counter).copied().unwrap_or(0)
            } else {
                totals.get(counter).copied().unwrap_or(0)
            };
            let tiers = template["tiers"].as_array().map(Vec::as_slice).unwrap_or_default();
            if tiers.is_empty() {
                if earned.contains_key(id)
                    || value < int(template, "target")
                    || !career::add_achievement(self.db, &key(user), id, now).await?
                {
                    continue;
                }
                career::pay(self.db, &key(user), template, now).await?;
                result.coins += int(template, "coins");
                result.xp += int(template, "xp");
                let mut with = earned.clone();
                with.insert(id.to_owned(), now);
                result.achievements.push(career::achievement(template, &with, value).ok_or(Stop::Decline)?);
                continue;
            }
            for (index, step) in tiers.iter().enumerate() {
                let tier = index as i64 + 1;
                let stored = format!("{id}:{tier}");
                if earned.contains_key(&stored) {
                    continue;
                }
                if value < int(step, "target") || !career::add_achievement(self.db, &key(user), &stored, now).await? {
                    break;
                }
                earned.insert(stored.clone(), now);
                career::pay(self.db, &key(user), step, now).await?;
                let tokens = int(step, "tokens");
                if tokens > 0 {
                    self.add_tokens(user, tokens, &format!("achievement {stored}"), now).await?;
                }
                result.coins += int(step, "coins");
                result.xp += int(step, "xp");
                result.tokens += tokens;
                let mut view = career::achievement(template, &earned, value).ok_or(Stop::Decline)?;
                // the tier this game reached, as the rewards show it
                let object = view.as_object_mut().ok_or(Stop::Decline)?;
                object.insert("tier".into(), json!(tier));
                object.insert("target".into(), json!(int(step, "target")));
                match step.get("text").filter(|t| !t.is_null()) {
                    Some(text) => object.insert("text".into(), text.clone()),
                    None => object.remove("text"),
                };
                object.insert("coins".into(), json!(int(step, "coins")));
                object.insert("xp".into(), json!(int(step, "xp")));
                match step.get("cosmetic").filter(|c| !c.is_null()) {
                    Some(cosmetic) => object.insert("cosmetic".into(), cosmetic.clone()),
                    None => object.remove("cosmetic"),
                };
                object.insert("tokens".into(), json!(tokens));
                result.achievements.push(reorder(view));
            }
        }
        Ok(())
    }

    /// `CareerProgress.levelsIn`
    async fn levels_in(&mut self, user: &str, xp_before: i64, result: &mut Result_, now: i64) -> Result<(), Stop> {
        let xp = self.xp(user).await?.unwrap_or(0);
        let level = career::xp_into_level(xp).0;
        result.level_before = career::xp_into_level(xp_before).0;
        result.level_after = level;
        let paid: i64 = sqlx::query_scalar("SELECT level_paid FROM profile WHERE user = ?")
            .bind(key(user))
            .fetch_optional(&mut *self.db)
            .await?
            .unwrap_or(0);
        let paid = paid.max(1);
        for next in paid + 1..=level {
            let Some(reward) = self.book.content.levels.iter().find(|r| int(r, "level") == next).cloned() else {
                continue;
            };
            let (coins, packs, tokens) = (int(&reward, "coins"), int(&reward, "packs"), int(&reward, "tokens"));
            if coins > 0 {
                self.add_coins(user, coins).await?;
            }
            if packs > 0 {
                self.add_profile_int(user, "pack_tokens", packs).await?;
            }
            if let Some(wildcard) = reward.get("wildcard").and_then(Value::as_str) {
                let column = match wildcard {
                    "common" => "wc_common",
                    "uncommon" => "wc_uncommon",
                    "rare" => "wc_rare",
                    "mythic" => "wc_mythic",
                    _ => return Err(Stop::Decline),
                };
                self.add_profile_int(user, column, 1).await?;
            }
            if let (Some(kind), Some(id)) =
                (reward["cosmetic"]["kind"].as_str(), reward["cosmetic"]["id"].as_str())
            {
                self.add_unlock(user, kind, id, now).await?;
            }
            if tokens > 0 {
                self.add_tokens(user, tokens, &format!("level {next}"), now).await?;
            }
            result.coins += coins;
            result.packs += packs;
            result.tokens += tokens;
            result.level_rewards.push(reward);
        }
        if level > paid {
            self.exec("UPDATE profile SET level_paid = ? WHERE user = ?", &[Bind::Int(level), Bind::Text(&key(user))])
                .await?;
        }
        let renown_xp = self.book.content.renown_xp;
        result.renown_before = career::renown_for(xp_before, renown_xp);
        result.renown_after = career::renown_for(xp, renown_xp);
        let renown_paid = self.stats(user).await?.get("renown").copied().unwrap_or(0);
        if result.renown_after > renown_paid {
            self.add_profile_int(user, "pack_tokens", result.renown_after - renown_paid).await?;
            result.packs += result.renown_after - renown_paid;
            self.max_stat(user, "renown", result.renown_after).await?;
        }
        let (_, into, next) = career::xp_into_level(xp);
        result.xp_into_level = into;
        result.xp_for_next = next;
        Ok(())
    }

    /// `CareerService.opponents`, the tiers open
    async fn open_tiers(&mut self, user: &str) -> Result<Vec<(i64, String)>, Stop> {
        let wins = career::wins_by_opponent(self.db, &key(user)).await?;
        let nemesis = self.nemeses.get(user).await?;
        let roster = career::opponents(self.book.roster.clone(), &wins, nemesis.as_deref()).ok_or(Stop::Decline)?;
        Ok(roster
            .as_array()
            .into_iter()
            .flatten()
            .filter(|view| view["unlocked"] == json!(true))
            .map(|view| (int(view, "tier"), view["tierName"].as_str().unwrap_or_default().to_owned()))
            .collect())
    }

    // ---- a roster duel (CareerService.payMatch)

    async fn match_ended(&mut self, event: &Event) -> Result<bool, Stop> {
        let table = event.career.as_ref().ok_or(Stop::Decline)?;
        let (won, turns, now) = (event.won, event.turns, event.at);
        let opponent = table.opponent.clone().unwrap_or_default();
        // CareerRules.reward, then assistedReward
        let (mut coins, mut xp, mut note) = if won {
            let win = match table.tier {
                1 => 25,
                2 => 30,
                3 => 35,
                4 => 45,
                _ => 60,
            };
            (win, 100 + 25 * (table.tier - 1), None)
        } else if turns < MIN_TURN_FOR_REWARD {
            (0, 0, Some(format!("The game ended before turn {MIN_TURN_FOR_REWARD}, so it paid nothing.")))
        } else {
            (5, 25, None)
        };
        if let Some(assisted) = &table.assisted {
            let help = format!("You had help ({assisted}), so this game pays half and counts for no quests.");
            coins /= 2;
            xp /= 2;
            note = Some(match note {
                None => help,
                Some(note) => format!("{note} {help}"),
            });
        }
        let user = table.user.as_str();
        let Some(xp_before) = self.xp(user).await? else { return Ok(false) };
        let inserted = self
            .exec(
                "INSERT OR IGNORE INTO payouts (match_key, user, at, opponent, won, coins, xp, note) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                &[
                    Bind::Text(&event.key),
                    Bind::Text(&key(user)),
                    Bind::Int(now),
                    Bind::Text(&opponent),
                    Bind::Int(i64::from(won)),
                    Bind::Int(coins),
                    Bind::Int(xp),
                    note.as_deref().map_or(Bind::Null, Bind::Text),
                ],
            )
            .await?;
        if inserted != 1 {
            return Ok(false);
        }
        self.exec(
            "UPDATE profile SET coins = coins + ?, xp = xp + ?, wins = wins + ?, losses = losses + ? WHERE user = ?",
            &[Bind::Int(coins), Bind::Int(xp), Bind::Int(i64::from(won)), Bind::Int(i64::from(!won)), Bind::Text(&key(user))],
        )
        .await?;
        self.claim_game(&event.key, user, now).await?;
        let mut result = Result_ {
            game_key: event.key.clone(),
            kind: "career".into(),
            career: true,
            won,
            opponent: Some(opponent.clone()),
            coins,
            xp,
            note: note.clone(),
            assisted: table.assisted.clone(),
            xp_before,
            ..Result_::default()
        };
        let tiers_before: HashSet<i64> = self.open_tiers(user).await?.into_iter().map(|(tier, _)| tier).collect();
        self.ladder_finished(user, &opponent, won, &mut result, now).await?;
        if turns >= self.turn_floor() {
            self.daily_bonuses(user, "career", won, &mut result).await?;
        }
        self.apply(user, &event.counters, &event.moments, &mut result, xp_before, now).await?;
        for (tier, name) in self.open_tiers(user).await? {
            if !tiers_before.contains(&tier) && !result.opened.contains(&name) {
                result.opened.push(name);
            }
        }
        let reason = note.unwrap_or_else(|| format!("{} {opponent}", table.kind));
        self.set_game(&result, &reason).await?;
        Ok(true)
    }

    /// `CareerLadder.finished`
    async fn ladder_finished(
        &mut self,
        user: &str,
        opponent_id: &str,
        won: bool,
        result: &mut Result_,
        now: i64,
    ) -> Result<(), Stop> {
        let Some(entry) =
            self.book.roster.as_array().into_iter().flatten().find(|e| e["view"]["id"] == json!(opponent_id)).cloned()
        else {
            return Ok(());
        };
        let view = &entry["view"];
        let name = view["name"].as_str().unwrap_or_default().to_owned();
        self.add_stat(user, &format!("met_{opponent_id}"), 1).await?;
        let mut mode = Mode {
            kind: "duel".into(),
            title: name.clone(),
            lines: Vec::new(),
            coins: 0,
            xp: 0,
            reference: Some(opponent_id.to_owned()),
        };
        let nemesis = self.nemeses.get(user).await?;
        let was_nemesis = nemesis.as_deref() == Some(opponent_id);
        let rival = view.get("rivalChapter").and_then(Value::as_i64);
        if won {
            if was_nemesis {
                self.nemeses.put(user, "null", now).await?;
                self.add_coins(user, NEMESIS_BOUNTY).await?;
                result.coins += NEMESIS_BOUNTY;
                mode.coins += NEMESIS_BOUNTY;
                mode.lines.push(format!("Nemesis beaten: {NEMESIS_BOUNTY} coins of bounty."));
                self.add_stat(user, "nemeses_beaten", 1).await?;
            }
            if let Some(chapter) = rival {
                let after = entry.get("rivalAfter").and_then(Value::as_str);
                mode.lines.push(format!("{name}: “{}”", after.unwrap_or("null")));
                self.max_stat(user, "rival_chapter", chapter).await?;
            }
        } else if view["raid"] != json!(true) {
            let next = json!({"opponentId": opponent_id, "since": now}).to_string();
            self.nemeses.put(user, &next, now).await?;
            if !was_nemesis {
                mode.lines.push(format!(
                    "{name} is your nemesis now: beat them for a bounty, but they'll start with {NEMESIS_LIFE} extra life."
                ));
            }
        }
        result.mode = (!mode.lines.is_empty()).then_some(mode);
        Ok(())
    }

    // ---- a game outside Career tables (CareerService.payGame)

    async fn game_ended(&mut self, event: &Event) -> Result<bool, Stop> {
        let finished = event.finished.as_ref().ok_or(Stop::Decline)?;
        let now = event.at;
        let day = self.day.clone();
        let capped = finished.kind == "person" || finished.kind == "pod";
        let (floor, cap) = (
            self.turn_floor(),
            self.rewards().get("pairCapPerDay").and_then(Value::as_i64).unwrap_or(5),
        );
        let humans = &finished.seats;
        let mut paid = false;
        for (index, seat) in humans.iter().enumerate() {
            if !seat.counted {
                continue;
            }
            let user = seat.user.as_str();
            let game_key = format!("{}/{}", event.key, key(user));
            self.ensure_profile(user, now).await?;
            if !self.claim_game(&game_key, user, now).await? {
                continue;
            }
            paid = true;
            let xp_before = self.xp(user).await?.unwrap_or(0);
            let mut result = Result_ {
                game_key: game_key.clone(),
                kind: finished.kind.clone(),
                won: seat.won,
                xp_before,
                ..Result_::default()
            };
            if finished.kind == "person" && humans.len() == 2 {
                result.opponent = Some(humans[if index == 0 { 1 } else { 0 }].user.clone());
            }
            let mut reason = None;
            if finished.turns < floor {
                reason = Some(format!("The game ended before turn {floor}, so it paid nothing."));
            } else if capped {
                for (other_index, other) in humans.iter().enumerate() {
                    if other_index != index && self.pair_games(&day, &pair(user, &other.user)).await? >= cap {
                        reason = Some(format!(
                            "You've played {} {cap} times today; more games together pay nothing until tomorrow.",
                            other.user
                        ));
                        break;
                    }
                }
            }
            if let Some(reason) = reason {
                result.note = Some(reason.clone());
                self.levels_in(user, xp_before, &mut result, now).await?;
                // the record says nothing was paid (setGameRecord with 0 and 0)
                self.set_game_raw(&game_key, &result.json(), &finished.kind, seat.won, 0, 0, &reason).await?;
                continue;
            }
            let mut xp = self.game_xp(&finished.kind, seat.won, finished.players, finished.ai_skill);
            let bo3_xp = self.scale_xp(int(self.rewards(), "bo3MatchWinXp"));
            if finished.bo3 && seat.match_won && bo3_xp > 0 {
                xp += bo3_xp;
                result.bonuses.push(bonus("You won the match", bo3_xp, 0, 0));
            }
            self.add_profile_int(user, "xp", xp).await?;
            result.xp = xp;
            self.daily_bonuses(user, &finished.kind, seat.won, &mut result).await?;
            self.apply(user, &seat.counters, &seat.moments, &mut result, xp_before, now).await?;
            let reason =
                format!("{}{} on turn {}", finished.kind, if seat.won { " win" } else { " loss" }, finished.turns);
            self.set_game(&result, &reason).await?;
        }
        if capped && finished.turns >= floor {
            for i in 0..humans.len() {
                for j in i + 1..humans.len() {
                    self.exec(
                        "INSERT INTO pair_games (day, pair, games) VALUES (?, ?, 1) ON CONFLICT (day, pair) DO UPDATE SET games = games + 1",
                        &[Bind::Text(&day), Bind::Text(&pair(&humans[i].user, &humans[j].user))],
                    )
                    .await?;
                }
            }
        }
        Ok(paid)
    }

    #[allow(clippy::too_many_arguments)]
    async fn set_game_raw(
        &mut self,
        game_key: &str,
        details: &Value,
        kind: &str,
        won: bool,
        coins: i64,
        xp: i64,
        reason: &str,
    ) -> Result<(), Stop> {
        let text = details.to_string();
        self.exec("UPDATE progress_games SET details = ? WHERE game_key = ?", &[Bind::Text(&text), Bind::Text(game_key)])
            .await?;
        self.exec(
            "UPDATE progress_games SET kind = ?, won = ?, coins = ?, xp = ?, reason = ? WHERE game_key = ?",
            &[
                Bind::Text(kind),
                Bind::Int(i64::from(won)),
                Bind::Int(coins),
                Bind::Int(xp),
                Bind::Text(reason),
                Bind::Text(game_key),
            ],
        )
        .await?;
        Ok(())
    }

    async fn pair_games(&mut self, day: &str, pair: &str) -> Result<i64, Stop> {
        Ok(sqlx::query_scalar("SELECT games FROM pair_games WHERE day = ? AND pair = ?")
            .bind(day)
            .bind(pair)
            .fetch_optional(&mut *self.db)
            .await?
            .unwrap_or(0))
    }

    /// `CareerRewards.gameXp`
    fn game_xp(&self, kind: &str, won: bool, players: i64, ai_skill: i64) -> i64 {
        let Some(rates) = self.rewards().get("kinds").and_then(|k| k.get(kind)) else { return 0 };
        let rate = |field: &str| rates.get(field).and_then(Value::as_i64).unwrap_or(0) as f64;
        let mut xp = if won { rate("winXp") } else { rate("lossXp") };
        if kind == "pod" && won {
            xp += rate("perExtraPlayerXp") * (players - 2).max(0) as f64;
        }
        if kind == "ai" && ai_skill > 0 {
            xp *= self
                .rewards()
                .get("aiSkill")
                .and_then(|skills| skills.get(ai_skill.to_string()))
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
        }
        // Math.round
        self.scale_xp((xp + 0.5).floor() as i64)
    }
}

enum Bind<'a> {
    Int(i64),
    Text(&'a str),
    Null,
}

/// `CareerService.pair`
fn pair(a: &str, b: &str) -> String {
    let (x, y) = (key(a), key(b));
    if x <= y {
        format!("{x}|{y}")
    } else {
        format!("{y}|{x}")
    }
}

/// `CareerProgress.moment`: a feat as the rewards tell it
fn moment_view(moment: &Moment) -> Value {
    let value = moment.value;
    let text = match moment.id.as_str() {
        "big_hit" => format!("Hit for {value} in one blow"),
        "storm" => format!("Cast {value} spells in one turn"),
        "mana_surge" => format!("Made {value} mana in one turn"),
        "life_swing" => format!("Gained {value} life at once"),
        "elimination" if value == 1 => "Knocked a player out".to_owned(),
        "elimination" => format!("Knocked out {value} players"),
        "commander_hit" => format!("Your commander hit for {value}"),
        "one_life" => "Won at 1 life".to_owned(),
        "comeback" => format!("Came back from {value} life"),
        "flawless" => "Won without taking damage".to_owned(),
        "fast_win" => format!("Won on your turn {value}"),
        other => other.to_owned(),
    };
    json!({"id": moment.id, "text": text, "value": value, "turn": moment.turn})
}

/// a `CareerAchievement` with its fields back in Gson's order after the tier's were set
fn reorder(view: Value) -> Value {
    const ORDER: [&str; 20] = [
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
        "holders",
        "",
    ];
    let Value::Object(mut fields) = view else { return view };
    let mut out = Map::new();
    for field in ORDER.iter().filter(|f| !f.is_empty()) {
        if let Some(value) = fields.remove(*field) {
            out.insert((*field).into(), value);
        }
    }
    out.extend(fields);
    Value::Object(out)
}

/// `CareerProgress.refreshQuests` and `refreshWeeklyQuest`, with the seeded dice: the day's quests and the week's
pub(crate) async fn refresh_quests(
    db: &mut SqliteConnection,
    book: &Book<'_>,
    user: &str,
    day: &str,
    week: &str,
) -> Result<(), Stop> {
    let user = key(user);
    let rows = async |db: &mut SqliteConnection| -> Result<Vec<(i64, String)>, Stop> {
        let mut rows = Vec::new();
        for row in sqlx::query("SELECT slot, quest FROM quests WHERE user = ? ORDER BY slot")
            .bind(&user)
            .fetch_all(&mut *db)
            .await?
        {
            rows.push((row.try_get::<i64, _>(0)?, row.try_get::<String, _>(1)?));
        }
        Ok(rows)
    };
    let text = async |db: &mut SqliteConnection, column: &str| -> Result<Option<String>, Stop> {
        let found: Option<Option<String>> = sqlx::query_scalar(&format!("SELECT {column} FROM profile WHERE user = ?"))
            .bind(&user)
            .fetch_optional(&mut *db)
            .await?;
        Ok(found.flatten())
    };
    let put = async |db: &mut SqliteConnection, slot: i64, quest: &str| -> Result<(), Stop> {
        sqlx::query("INSERT OR REPLACE INTO quests (user, slot, quest, progress, assigned) VALUES (?, ?, ?, 0, ?)")
            .bind(&user)
            .bind(slot)
            .bind(quest)
            .bind(day)
            .execute(&mut *db)
            .await?;
        Ok(())
    };
    // the weekly slot
    if text(db, "quest_week").await?.as_deref() != Some(week) {
        let present: HashSet<String> = rows(db).await?.into_iter().map(|(_, quest)| quest).collect();
        if let Some(quest) = pick_quest(book, &present, true, &format!("weekly/{user}/{week}")) {
            put(db, WEEKLY_SLOT, &quest).await?;
        }
        sqlx::query("UPDATE profile SET quest_week = ? WHERE user = ?").bind(week).bind(&user).execute(&mut *db).await?;
    }
    let last = text(db, "quest_day").await?;
    if last.as_deref() == Some(day) {
        return Ok(());
    }
    let waiting: Vec<(i64, String)> = rows(db).await?.into_iter().filter(|(slot, _)| *slot != WEEKLY_SLOT).collect();
    let free = QUEST_SLOTS - waiting.len() as i64;
    let to_add = if last.is_none() { free } else { free.min(1) };
    let mut used: HashSet<i64> = waiting.iter().map(|(slot, _)| *slot).collect();
    let mut present: HashSet<String> = waiting.into_iter().map(|(_, quest)| quest).collect();
    for _ in 0..to_add {
        let mut slot = 0;
        while used.contains(&slot) {
            slot += 1;
        }
        let Some(quest) = pick_quest(book, &present, false, &format!("quest/{user}/{day}/{slot}")) else { break };
        put(db, slot, &quest).await?;
        used.insert(slot);
        present.insert(quest);
    }
    sqlx::query("UPDATE profile SET quest_day = ? WHERE user = ?").bind(day).bind(&user).execute(&mut *db).await?;
    Ok(())
}

/// `CareerProgress.pickQuest`: one of the templates not present, of the kind asked, by the purpose's roll
pub(crate) fn pick_quest(book: &Book<'_>, exclude: &HashSet<String>, weekly: bool, purpose: &str) -> Option<String> {
    let choices: Vec<&str> = book
        .quests
        .as_array()?
        .iter()
        .filter(|t| t["weekly"].as_bool().unwrap_or(false) == weekly)
        .filter_map(|t| t["id"].as_str())
        .filter(|id| !exclude.contains(*id))
        .collect();
    if choices.is_empty() {
        return None;
    }
    let index = dice::roll(book.secret, purpose).next_int(choices.len() as i32);
    Some(choices[index as usize].to_owned())
}

// ---- the rows around a payout, for shadow (CareerStore.snapshot)

/// `CareerStore.PAYOUT_TABLES`
const PAYOUT_TABLES: [&str; 9] =
    ["profile", "stats", "quests", "achievements", "unlocks", "weekly", "daily", "payouts", "progress_games"];

/// a row as Java's `getObject` and Gson write it: whole numbers, text or null
fn row_json(row: &sqlx::sqlite::SqliteRow) -> Value {
    use sqlx::{Column, ValueRef};
    let mut out = Map::new();
    for (index, column) in row.columns().iter().enumerate() {
        let raw = row.try_get_raw(index).ok();
        let value = if raw.as_ref().is_none_or(|r| r.is_null()) {
            Value::Null
        } else if let Ok(number) = row.try_get::<i64, _>(index) {
            json!(number)
        } else if let Ok(text) = row.try_get::<String, _>(index) {
            json!(text)
        } else {
            Value::Null
        };
        out.insert(column.name().to_owned(), value);
    }
    Value::Object(out)
}

/// the payout's rows for these accounts and the day's pair games, as Java's snapshot holds them
pub(crate) async fn snapshot(db: &mut SqliteConnection, users: &[String], day: &str) -> Result<Map<String, Value>, Stop> {
    let mut keys: Vec<String> = Vec::new();
    for user in users {
        if !keys.contains(&key(user)) {
            keys.push(key(user));
        }
    }
    let mut out = Map::new();
    for table in PAYOUT_TABLES {
        let mut rows = Vec::new();
        for user in &keys {
            for row in sqlx::query(&format!("SELECT * FROM {table} WHERE user = ?")).bind(user).fetch_all(&mut *db).await?
            {
                rows.push(row_json(&row));
            }
        }
        out.insert(table.into(), Value::Array(rows));
    }
    let pairs: Vec<Value> = sqlx::query("SELECT * FROM pair_games WHERE day = ?")
        .bind(day)
        .fetch_all(&mut *db)
        .await?
        .iter()
        .map(row_json)
        .collect();
    out.insert("pair_games".into(), Value::Array(pairs));
    Ok(out)
}

/// A copy of career.db holding only the rows a payout needs: Java's schema, its secret and the snapshot
pub(crate) async fn copy(schema: &[String], secret: &str, rows: &Map<String, Value>) -> Result<SqliteConnection, Stop> {
    use sqlx::ConnectOptions;
    let mut db = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).connect().await?;
    for sql in schema {
        sqlx::query(sql).execute(&mut db).await?;
    }
    sqlx::query("INSERT OR REPLACE INTO meta (key, value) VALUES ('secret', ?)").bind(secret).execute(&mut db).await?;
    for (table, list) in rows {
        if table == "nemesis" {
            continue;
        }
        if !table.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            return Err(Stop::Decline);
        }
        for row in list.as_array().into_iter().flatten() {
            let Some(row) = row.as_object() else { continue };
            let columns: Vec<&String> = row.keys().collect();
            if !columns.iter().all(|c| c.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')) {
                return Err(Stop::Decline);
            }
            let sql = format!(
                "INSERT INTO {table} ({}) VALUES ({})",
                columns.iter().map(|c| c.as_str()).collect::<Vec<_>>().join(", "),
                vec!["?"; columns.len()].join(", ")
            );
            let mut query = sqlx::query(&sql);
            for column in &columns {
                query = match &row[column.as_str()] {
                    Value::Null => query.bind(Option::<i64>::None),
                    Value::Number(n) => query.bind(n.as_i64().ok_or(Stop::Decline)?),
                    Value::String(s) => query.bind(s.clone()),
                    Value::Bool(b) => query.bind(i64::from(*b)),
                    _ => return Err(Stop::Decline),
                };
            }
            query.execute(&mut db).await?;
        }
    }
    Ok(db)
}

/// The differences between Java's rows after a payout and Rust's: by table, rows matched as sets, a game's result
/// compared as JSON (Gson and serde escape differently). None when they're the same.
pub fn difference(java: &Map<String, Value>, rust: &Map<String, Value>) -> Option<String> {
    let normal = |rows: Option<&Value>| -> Vec<String> {
        let mut list: Vec<String> = rows
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|row| {
                let mut row = row.clone();
                // Gson leaves null columns out of the rows Java kept
                if let Some(fields) = row.as_object_mut() {
                    fields.retain(|_, value| !value.is_null());
                }
                if let Some(details) = row.get("details").and_then(Value::as_str) {
                    if let Ok(parsed) = serde_json::from_str::<Value>(details) {
                        row["details"] = parsed;
                    }
                }
                row.to_string()
            })
            .collect();
        list.sort();
        list
    };
    let mut tables: Vec<&String> = java.keys().chain(rust.keys()).filter(|t| *t != "nemesis").collect();
    tables.sort();
    tables.dedup();
    let mut problems = Vec::new();
    for table in tables {
        let (a, b) = (normal(java.get(table)), normal(rust.get(table)));
        if a != b {
            let only_java: Vec<&String> = a.iter().filter(|r| !b.contains(r)).collect();
            let only_rust: Vec<&String> = b.iter().filter(|r| !a.contains(r)).collect();
            problems.push(format!("{table}: java {only_java:?} rust {only_rust:?}"));
        }
    }
    if java.get("nemesis") != rust.get("nemesis") {
        problems.push(format!("nemesis: java {:?} rust {:?}", java.get("nemesis"), rust.get("nemesis")));
    }
    (!problems.is_empty()).then(|| problems.join("; "))
}

/// Replays an event Java paid on a copy of the rows from before, and compares the rows after (shadow).
/// Ok(None) when they're the same.
pub(crate) async fn replay(
    book: &Book<'_>,
    schema: &[String],
    event: &Event,
    before: &Map<String, Value>,
    after: &Map<String, Value>,
) -> Result<Option<String>, Stop> {
    let mut db = copy(schema, book.secret, before).await?;
    let nemesis: HashMap<String, Option<String>> = before
        .get("nemesis")
        .and_then(Value::as_object)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_str().map(str::to_owned))).collect())
        .unwrap_or_default();
    let mut nemeses = Nemeses::Copy(nemesis);
    pay(&mut db, book, &mut nemeses, event).await?;
    let mut rows = snapshot(&mut db, &event.users(), &event.day).await?;
    if let Nemeses::Copy(map) = &nemeses {
        if before.contains_key("nemesis") {
            rows.insert("nemesis".into(), json!(map));
        }
    }
    Ok(difference(after, &rows))
}

// ---- the outbox (payout_events)

/// `XMAGE_ROUTE_CAREERPAYOUTS`: who pays a finished game
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayoutRoute {
    /// Java pays; the gateway does nothing
    Java,
    /// Java pays and keeps the rows around it; the gateway replays each payout on a copy and compares
    Shadow,
    /// the gateway pays the events Java leaves pending
    Rust,
}

/// what one pass did: each shadow comparison (`matchEnded` or `gameEnded`, same?), and the events paid
#[derive(Debug, Default)]
pub struct Pass {
    pub compared: Vec<(String, bool)>,
    pub paid: usize,
}

impl crate::admin::AdminStores {
    /// career.db's secret (`CareerStore.secret`), made here when Java hasn't yet
    async fn secret(db: &mut SqliteConnection) -> Result<String, Stop> {
        let found: Option<String> =
            sqlx::query_scalar("SELECT value FROM meta WHERE key = 'secret'").fetch_optional(&mut *db).await?;
        if let Some(secret) = found {
            return Ok(secret);
        }
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).map_err(|_| Stop::Decline)?;
        sqlx::query("INSERT OR IGNORE INTO meta (key, value) VALUES ('secret', ?)")
            .bind(base64(&bytes))
            .execute(&mut *db)
            .await?;
        Ok(sqlx::query_scalar("SELECT value FROM meta WHERE key = 'secret'").fetch_one(&mut *db).await?)
    }

    /// One pass over `payout_events`: on `rust` pays what's pending, on `shadow` replays what Java paid and hasn't
    /// been checked. `shadow_users`: when set, only these accounts' payouts are replayed (`XMAGE_SHADOW_USERS`).
    pub async fn payouts_pass(&self, route: PayoutRoute, shadow_users: Option<&[String]>) -> Pass {
        let mut pass = Pass::default();
        let (Some(pool), Some(content), Some(views)) = (&self.career, &self.career_content, &self.career_views) else {
            return pass;
        };
        let (Some(quests), Some(achievements), Some(roster), Some(cosmetics)) =
            (views.view("quests"), views.view("achievements"), views.view("opponents"), views.view("cosmetics"))
        else {
            return pass;
        };
        let Ok(mut db) = pool.acquire().await else { return pass };
        // before the payout_events table (Java's migration 7) there's nothing to do
        let ready: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'payout_events'")
                .fetch_optional(&mut *db)
                .await
                .unwrap_or(None);
        if ready.is_none() {
            return pass;
        }
        let Ok(secret) = Self::secret(&mut db).await else { return pass };
        let book = Book {
            content,
            quests: &quests,
            achievements: &achievements,
            roster: &roster,
            cosmetics: &cosmetics,
            secret: &secret,
        };
        match route {
            PayoutRoute::Java => {}
            PayoutRoute::Rust => {
                let pending: Vec<(String, String)> = sqlx::query_as(
                    "SELECT key, event FROM payout_events WHERE state = 'pending' ORDER BY at, key LIMIT 50",
                )
                .fetch_all(&mut *db)
                .await
                .unwrap_or_default();
                for (key, text) in pending {
                    let Ok(event) = serde_json::from_str::<Event>(&text) else {
                        tracing::warn!("Career payout {key}: the event can't be read");
                        continue;
                    };
                    let Some(solo) = &self.solo else { continue };
                    let mut nemeses = Nemeses::Solo(solo.clone());
                    if sqlx::query("BEGIN IMMEDIATE").execute(&mut *db).await.is_err() {
                        continue;
                    }
                    let still: Option<String> = sqlx::query_scalar("SELECT state FROM payout_events WHERE key = ?")
                        .bind(&key)
                        .fetch_optional(&mut *db)
                        .await
                        .unwrap_or(None);
                    let outcome = if still.as_deref() != Some("pending") {
                        Err(Stop::Decline)
                    } else {
                        pay(&mut db, &book, &mut nemeses, &event).await
                    };
                    let marked = match &outcome {
                        Ok(_) => sqlx::query(
                            "UPDATE payout_events SET state = 'paid', paid_by = 'rust', paid_at = ? WHERE key = ?",
                        )
                        .bind(crate::admin::now_millis())
                        .bind(&key)
                        .execute(&mut *db)
                        .await
                        .is_ok(),
                        Err(_) => false,
                    };
                    let end = if marked { "COMMIT" } else { "ROLLBACK" };
                    if sqlx::query(end).execute(&mut *db).await.is_ok() && marked {
                        pass.paid += 1;
                        tracing::info!("Career payout {key} ({}) paid by Rust for {:?}", event.event, event.users());
                    } else if still.as_deref() == Some("pending") {
                        tracing::warn!("Career payout {key} ({}): Rust can't pay it; it waits", event.event);
                    }
                }
            }
            PayoutRoute::Shadow => {
                let schema: Vec<String> = sqlx::query_scalar(
                    "SELECT sql FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY type DESC, name",
                )
                .fetch_all(&mut *db)
                .await
                .unwrap_or_default();
                let rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
                    "SELECT key, event, before, after FROM payout_events \
                     WHERE state = 'paid' AND paid_by = 'java' AND before IS NOT NULL AND checked IS NULL ORDER BY at LIMIT 20",
                )
                .fetch_all(&mut *db)
                .await
                .unwrap_or_default();
                for (key, text, before, after) in rows {
                    let parsed = (
                        serde_json::from_str::<Event>(&text),
                        serde_json::from_str::<Map<String, Value>>(&before),
                        after.as_deref().map(serde_json::from_str::<Map<String, Value>>),
                    );
                    let checked = match parsed {
                        (Ok(event), Ok(before), Some(Ok(after))) => {
                            let listed = shadow_users.is_none_or(|users| {
                                event.users().iter().any(|user| users.contains(&user.to_lowercase()))
                            });
                            if !listed {
                                "skipped".to_owned()
                            } else {
                                match replay(&book, &schema, &event, &before, &after).await {
                                    Ok(None) => {
                                        pass.compared.push((event.event.clone(), true));
                                        "same".to_owned()
                                    }
                                    Ok(Some(difference)) => {
                                        tracing::warn!("Career payout {key} ({}): Rust differs: {difference}", event.event);
                                        pass.compared.push((event.event.clone(), false));
                                        format!("different: {difference}")
                                    }
                                    Err(_) => "declined".to_owned(),
                                }
                            }
                        }
                        _ => "unreadable".to_owned(),
                    };
                    let _ = sqlx::query("UPDATE payout_events SET checked = ? WHERE key = ?")
                        .bind(checked)
                        .bind(&key)
                        .execute(&mut *db)
                        .await;
                }
            }
        }
        pass
    }
}

/// standard base64, as Java's `Base64.getEncoder()` writes it
fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../Mage.Server/src/main/resources/career")
    }

    /// Java's payouts (CareerPayoutParityTest, on the shadow route) replayed here: the same rows after, row for row
    #[tokio::test]
    async fn payouts_match_java() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/payouts/cases.json");
        let fixture: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let content = CareerContent::load(&content_dir()).unwrap();
        let book = &fixture["book"];
        let schema: Vec<String> = serde_json::from_value(book["schema"].clone()).unwrap();
        let book = Book {
            content: &content,
            quests: &book["quests"],
            achievements: &book["achievements"],
            roster: &book["opponents"],
            cosmetics: &book["cosmetics"],
            secret: book["secret"].as_str().unwrap(),
        };
        let cases = fixture["cases"].as_array().unwrap();
        assert!(cases.len() >= 14);
        let mut problems = Vec::new();
        for case in cases {
            let event: Event = serde_json::from_value(case["event"].clone()).unwrap();
            let (before, after) = (case["before"].as_object().unwrap(), case["after"].as_object().unwrap());
            match replay(&book, &schema, &event, before, after).await {
                Ok(None) => {}
                Ok(Some(difference)) => problems.push(format!("{}: {difference}", case["name"])),
                Err(_) => problems.push(format!("{}: declined", case["name"])),
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));

        // and a payout that differs is told: one coin more on Java's side
        let case = &cases[0];
        let event: Event = serde_json::from_value(case["event"].clone()).unwrap();
        let mut after = case["after"].as_object().unwrap().clone();
        after["profile"][0]["coins"] = json!(after["profile"][0]["coins"].as_i64().unwrap() + 1);
        let found = replay(&book, &schema, &event, case["before"].as_object().unwrap(), &after).await.unwrap();
        assert!(found.is_some_and(|d| d.starts_with("profile:")));
    }

    #[test]
    fn base64_as_java() {
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(base64(b"abc"), "YWJj");
        assert_eq!(base64(b"a"), "YQ==");
    }
}
