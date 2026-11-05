//! Shadow games (docs/rust/Engine.md): a game Java played, replayed here from the same start with the same answers.

use crate::card::CardDb;
use crate::decide::{Answer, Decider, Query};
use crate::fingerprint;
use crate::state::{Game, ObjId, Seat, Step, Zone};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct ShadowGame {
    pub version: u32,
    pub seed: i64,
    pub decks: Vec<ShadowDeck>,
    pub start: ShadowStart,
    pub decisions: Vec<ShadowDecision>,
    #[serde(default)]
    pub questions: Vec<String>,
    pub trace: Vec<ShadowEntry>,
    pub result: String,
    #[serde(default)]
    pub turns: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShadowDeck {
    pub name: String,
    pub main: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShadowStart {
    /// answers given before the first priority (mulligans)
    pub decision: usize,
    pub random: i64,
    pub seats: Vec<ShadowSeat>,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShadowSeat {
    pub library: Vec<String>,
    pub hand: Vec<String>,
    /// each card's place in Java's triggered-ability registration order (-1: no triggers); older games lack it
    #[serde(default, rename = "libraryTriggerOrder")]
    pub library_trigger_order: Vec<i64>,
    #[serde(default, rename = "handTriggerOrder")]
    pub hand_trigger_order: Vec<i64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ShadowDecision {
    pub seat: usize,
    pub target: Option<String>,
    pub bool: Option<bool>,
    pub integer: Option<i32>,
    pub string: Option<String>,
    #[serde(rename = "manaType")]
    pub mana_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShadowEntry {
    pub point: String,
    pub decision: Option<usize>,
    pub fp: String,
    pub text: Option<String>,
}

impl ShadowGame {
    pub fn parse(text: &str) -> Result<ShadowGame, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }

    /// Every card name in both decks.
    pub fn card_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for seat in &self.start.seats {
            for n in seat.library.iter().chain(seat.hand.iter()) {
                if !names.contains(n) {
                    names.push(n.clone());
                }
            }
        }
        names
    }

    /// The game at its first priority: the recorded libraries and hands, the turn and step of the first trace entry.
    pub fn start_game(&self, db: &CardDb) -> Result<Game, String> {
        let mut game = Game::new(self.start.seats.len(), self.start.random);
        for (seat, s) in self.start.seats.iter().enumerate() {
            for (i, name) in s.library.iter().enumerate() {
                let def = db.get(name).ok_or_else(|| format!("card not in the engine: {}", name))?;
                let id = game.create_obj(def, seat, Zone::Library);
                game.player_mut(seat).library.push(id);
                if let Some(o) = s.library_trigger_order.get(i).filter(|o| **o >= 0) {
                    game.obj_mut(id).trigger_order = *o as u64;
                }
            }
            for (i, name) in s.hand.iter().enumerate() {
                let def = db.get(name).ok_or_else(|| format!("card not in the engine: {}", name))?;
                let id = game.create_obj(def, seat, Zone::Hand);
                game.player_mut(seat).hand.push(id);
                if let Some(o) = s.hand_trigger_order.get(i).filter(|o| **o >= 0) {
                    game.obj_mut(id).trigger_order = *o as u64;
                }
            }
        }
        let header = self.start.fingerprint.lines().nth(1).ok_or("no fingerprint")?;
        // turn <n> step <STEP> active <seat> priority <seat>
        let words: Vec<&str> = header.split(' ').collect();
        game.turn = words.get(1).and_then(|w| w.parse().ok()).ok_or("no turn")?;
        game.step = words.get(3).and_then(|w| Step::parse(w));
        game.active = words.get(5).and_then(|w| w.parse().ok());
        game.priority = words.get(7).and_then(|w| w.parse().ok());
        Ok(game)
    }
}

/// What the replay found.
#[derive(Clone, Debug, Default)]
pub struct ReplayReport {
    /// trace entries that matched before the first difference
    pub matched: usize,
    pub entries: usize,
    /// answers used
    pub answers: usize,
    pub failure: Option<String>,
}

impl ReplayReport {
    pub fn complete(&self) -> bool {
        self.failure.is_none() && self.matched == self.entries
    }
}

/// Finds the object a recorded answer names, in the order Java's `DecisionLog.visit` enumerates objects.
pub fn resolve(game: &Game, r: &str) -> Option<Answer> {
    if r.starts_with('?') {
        return Some(Answer::Unknown);
    }
    // CHOOSE_MODE's Done and Cancel
    match r {
        "MD" => return Some(Answer::ModeDone),
        "MC" => return Some(Answer::Unknown),
        _ => {}
    }
    // a spell's mode: `<stack object>sm<index>`
    if let Some((_, mode)) = r.rsplit_once("sm") {
        if let Ok(k) = mode.parse::<usize>() {
            return Some(Answer::Mode(k));
        }
    }
    // an object's ability: `<object>a<index>`
    if let Some((obj, ability)) = r.rsplit_once('a') {
        if let (Some(Answer::Obj(o)), Ok(i)) = (resolve(game, obj), ability.parse::<usize>()) {
            return Some(Answer::Ability(o, i));
        }
    }
    let (kind, rest) = r.split_at(1);
    match kind {
        "P" => rest.parse::<Seat>().ok().filter(|s| *s < game.seats()).map(Answer::Player),
        "H" | "L" | "G" => {
            let (seat, index) = rest.split_once('.')?;
            let seat: Seat = seat.parse().ok()?;
            let index: usize = index.parse().ok()?;
            let player = game.players.get(seat)?;
            let zone = match kind {
                "H" => &player.hand,
                "L" => &player.library,
                _ => &player.graveyard,
            };
            zone.get(index).copied().map(Answer::Obj)
        }
        "B" => game.battlefield.get(rest.parse::<usize>().ok()?).copied().map(Answer::Obj),
        "S" => {
            let index: usize = rest.parse().ok()?;
            let n = game.stack.len();
            if index >= n {
                return None;
            }
            Some(Answer::Obj(game.stack[n - 1 - index].card))
        }
        "E" => game.exile.get(rest.parse::<usize>().ok()?).copied().map(Answer::Obj),
        // a waiting trigger: its seat, then its place among that seat's
        "T" => rest.split_once('.').and_then(|(_, i)| i.parse().ok()).map(Answer::Trigger),
        _ => None,
    }
}

/// Where an object is, as `DecisionLog.ref` writes it.
pub fn reference(game: &Game, id: ObjId) -> String {
    for (s, p) in game.players.iter().enumerate() {
        for (prefix, zone) in [("H", &p.hand), ("L", &p.library), ("G", &p.graveyard)] {
            if let Some(i) = zone.iter().position(|x| *x == id) {
                return format!("{}{}.{}", prefix, s, i);
            }
        }
    }
    if let Some(i) = game.battlefield.iter().position(|x| *x == id) {
        return format!("B{}", i);
    }
    if let Some(i) = game.stack.iter().rev().position(|x| x.card == id) {
        return format!("S{}", i);
    }
    if let Some(i) = game.exile.iter().position(|x| *x == id) {
        return format!("E{}", i);
    }
    format!("?{}", id.0)
}

/// Answers from the log, in order; records the trace the engine gives and stops at the first difference.
pub struct ReplayDecider<'a> {
    shadow: &'a ShadowGame,
    pub next: usize,
    pub trace: Vec<String>,
    pub failure: Option<String>,
}

impl<'a> ReplayDecider<'a> {
    pub fn new(shadow: &'a ShadowGame) -> Self {
        ReplayDecider { shadow, next: shadow.start.decision, trace: Vec::new(), failure: None }
    }

    fn fail(&mut self, why: String) {
        if self.failure.is_none() {
            self.failure = Some(why);
        }
    }
}

impl<'a> Decider for ReplayDecider<'a> {
    fn answer(&mut self, game: &Game, seat: Seat, query: &Query) -> Answer {
        if self.failure.is_some() {
            return Answer::Stop;
        }
        let Some(d) = self.shadow.decisions.get(self.next) else {
            self.fail(format!("the log ends after {} answers; asked {:?}", self.next, query));
            return Answer::Stop;
        };
        let asked = self.shadow.questions.get(self.next).cloned().unwrap_or_default();
        if d.seat != seat {
            self.fail(format!(
                "answer {} is seat {}'s ({}), but seat {} is asked {:?}",
                self.next, d.seat, asked, seat, query
            ));
            return Answer::Stop;
        }
        let answer = if let Some(r) = &d.target {
            match resolve(game, r) {
                Some(a) => a,
                None => {
                    self.fail(format!("answer {} names {}, which isn't in this game ({})", self.next, r, asked));
                    return Answer::Stop;
                }
            }
        } else if let Some(s) = &d.string {
            Answer::Text(s.clone())
        } else if let Some(i) = d.integer {
            Answer::Int(i)
        } else if let Some(m) = &d.mana_type {
            Answer::ManaType(m.clone())
        } else {
            Answer::Bool(d.bool.unwrap_or(false))
        };
        if !query.accepts(&answer) {
            self.fail(format!(
                "answer {} ({:?}, Java asked {}) doesn't fit the question {:?}",
                self.next, answer, asked, query
            ));
            return Answer::Stop;
        }
        if std::env::var_os("PLAYMAT_SHADOW_DEBUG").is_some() {
            eprintln!("answer {} seat {} {:?} -> {:?} ({})", self.next, seat, query, answer, asked);
        }
        self.next += 1;
        answer
    }

    fn traced(&mut self, game: &Game) {
        if self.failure.is_some() {
            return;
        }
        let text = fingerprint::text(game);
        let hash = fingerprint::hash_text(&text);
        if self.trace.last() == Some(&hash) {
            return;
        }
        let i = self.trace.len();
        match self.shadow.trace.get(i) {
            Some(e) if e.fp == hash => self.trace.push(hash),
            Some(e) => {
                self.fail(format!(
                    "trace entry {} ({}) differs after {} answers (Java: {:?})\n--- Java\n{}--- Rust\n{}",
                    i,
                    e.point,
                    self.next,
                    e.decision,
                    e.text.clone().unwrap_or_default(),
                    text
                ));
                self.trace.push(hash);
            }
            None => {
                self.fail(format!("the engine goes on after Java's {} trace entries\n--- Rust\n{}", i, text));
                self.trace.push(hash);
            }
        }
    }

    fn stopped(&self) -> bool {
        self.failure.is_some()
    }
}

/// Replays a shadow game and reports how far it matched.
pub fn replay(shadow: &ShadowGame, db: &CardDb) -> ReplayReport {
    let mut report = ReplayReport { entries: shadow.trace.len(), ..ReplayReport::default() };
    let mut game = match shadow.start_game(db) {
        Ok(g) => g,
        Err(e) => {
            report.failure = Some(e);
            return report;
        }
    };
    let mut decider = ReplayDecider::new(shadow);
    crate::rules::play_from_priority(&mut game, &mut decider, shadow.turns);
    report.matched = decider.trace.iter().zip(shadow.trace.iter()).take_while(|(a, b)| **a == b.fp).count();
    report.answers = decider.next;
    report.failure = decider.failure.clone();
    if report.failure.is_none() && report.matched < report.entries {
        report.failure = Some(format!(
            "the engine stopped after {} of Java's {} trace entries, {} of {} answers used",
            decider.trace.len(),
            shadow.trace.len(),
            decider.next,
            shadow.decisions.len()
        ));
    }
    report
}
