//! Shadow games (docs/rust/Engine.md): a game Java played, replayed here from the same start with the same answers.

use crate::card::CardDb;
use crate::decide::{Answer, Decider, Query};
use crate::fingerprint;
use crate::state::{Game, ObjId, Seat, Step, Zone};
use serde::Deserialize;
use std::sync::Arc;

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
    /// the asked player's view at some answers (`-Dxmage.shadowGames.views`, docs/rust/Views.md)
    #[serde(default)]
    pub views: Vec<ShadowView>,
    /// the prompt the client got at each question, in games with views (docs/rust/Views.md)
    #[serde(default)]
    pub prompts: Vec<ShadowPrompt>,
    /// the game log's lines, in games with views
    #[serde(default)]
    pub log: Vec<ShadowLog>,
}

/// What a replay calls at each recorded prompt's question: the game, the engine's question, Java's prompt
pub type OnPrompt<'a> = Box<dyn FnMut(&Game, &Query, &ShadowPrompt) + 'a>;

#[derive(Clone, Debug, Deserialize)]
pub struct ShadowPrompt {
    pub decision: usize,
    pub seat: Seat,
    /// `{method, data}`: the callback's name and what it carries but the game view, ids as references
    pub prompt: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShadowLog {
    /// the answers given before it
    pub decision: usize,
    pub line: String,
}

/// What a replay calls at each recorded view, with the engine's state at that point
pub type OnView<'a> = Box<dyn FnMut(&Game, &ShadowView) + 'a>;

#[derive(Clone, Debug, Deserialize)]
pub struct ShadowView {
    /// the answer the player was about to give
    pub decision: usize,
    pub seat: Seat,
    #[serde(default)]
    pub query: String,
    /// Java's `GameView` as the web client gets it, ids as references
    pub view: serde_json::Value,
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
    /// "start": exile is named in the cards' order at the first priority; older games name it as it was exiled
    #[serde(default, rename = "exileOrder")]
    pub exile_order: Option<String>,
    /// cards revealed from an opening hand for a delayed trigger (Devourer of Destiny), as `[seat, hand index]`;
    /// older games lack it, and their answers say instead
    #[serde(default)]
    pub revealed: Option<Vec<(usize, usize)>>,
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
    /// each card's printing, `SET:number` (newer games)
    #[serde(default, rename = "libraryPrintings")]
    pub library_printings: Vec<String>,
    #[serde(default, rename = "handPrintings")]
    pub hand_printings: Vec<String>,
    /// how many cards its sideboard has (newer games), for Learn's hint
    #[serde(default)]
    pub sideboard: usize,
    /// the sideboard's cards (newer games), named by answers as `O<seat>.<index>`: Learn's Lessons
    #[serde(default, rename = "sideboardCards")]
    pub sideboard_cards: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize, PartialEq)]
pub struct ShadowDecision {
    pub seat: usize,
    pub target: Option<String>,
    pub bool: Option<bool>,
    pub integer: Option<i32>,
    pub string: Option<String>,
    #[serde(rename = "manaType", skip_serializing_if = "Option::is_none")]
    pub mana_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShadowEntry {
    pub point: String,
    pub decision: Option<usize>,
    pub fp: String,
    pub text: Option<String>,
    /// Java's random generator's position there (`GameRandom.getState`), in recordings that have it
    #[serde(default)]
    pub random: Option<i64>,
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
        game.exile_by_creation = self.start.exile_order.as_deref() == Some("start");
        for (seat, s) in self.start.seats.iter().enumerate() {
            game.player_mut(seat).sideboard = s.sideboard;
            for (i, name) in s.library.iter().enumerate() {
                let def = db.get(name).ok_or_else(|| format!("card not in the engine: {}", name))?;
                let id = game.create_obj(def, seat, Zone::Library);
                game.player_mut(seat).library.push(id);
                set_printing(&mut game, id, s.library_printings.get(i));
                if let Some(o) = s.library_trigger_order.get(i).filter(|o| **o >= 0) {
                    game.obj_mut(id).trigger_order = *o as u64;
                }
            }
            for (i, name) in s.hand.iter().enumerate() {
                let def = db.get(name).ok_or_else(|| format!("card not in the engine: {}", name))?;
                let id = game.create_obj(def, seat, Zone::Hand);
                game.player_mut(seat).hand.push(id);
                set_printing(&mut game, id, s.hand_printings.get(i));
                if let Some(o) = s.hand_trigger_order.get(i).filter(|o| **o >= 0) {
                    game.obj_mut(id).trigger_order = *o as u64;
                }
            }
        }
        let mut gemstone_seat: Option<usize> = None;
        // permanents put onto the battlefield from opening hands (Leyline of the Void): `bf <seat> <name> U|T p/t dmg …`
        for line in self.start.fingerprint.lines().filter_map(|l| l.strip_prefix("bf ")) {
            let words: Vec<&str> = line.split(' ').collect();
            let Some(dmg) = words.iter().position(|w| *w == "dmg") else { continue };
            let mut end = dmg - 1;
            if words.get(end - 1) == Some(&"facedown") {
                end -= 1;
            }
            let tapped = words.get(end - 1) == Some(&"T");
            let seat: Seat = words[0].parse().map_err(|_| format!("bad battlefield line: {}", line))?;
            let name = words[1..end - 1].join(" ");
            let def = db.get(&name).ok_or_else(|| format!("card not in the engine: {}", name))?;
            let id = game.create_obj(def, seat, Zone::Battlefield);
            Arc::make_mut(&mut game.battlefield).push(id);
            game.obj_mut(id).tapped = tapped;
            // its counters (Gemstone Caverns' luck): `counters name=n,…`
            if let Some(c) = words.iter().position(|w| *w == "counters") {
                for kv in words.get(c + 1).unwrap_or(&"-").split(',') {
                    if let Some((k, n)) = kv.split_once('=') {
                        game.obj_mut(id).add_counters(k, n.parse().unwrap_or(0));
                    }
                }
                // a card exiled as it went there (Gemstone Caverns): the start's exile, its owner's
                if name == "Gemstone Caverns" {
                    gemstone_seat = Some(seat);
                }
            }
        }
        // the start's exile (only an opening hand's action puts cards there)
        if let Some(line) = self.start.fingerprint.lines().find_map(|l| l.strip_prefix("exile ")) {
            for name in line.split('|').filter(|n| !n.is_empty()) {
                let def = db.get(name).ok_or_else(|| format!("card not in the engine: {}", name))?;
                let id = game.create_obj(def, gemstone_seat.unwrap_or(0), Zone::Exile);
                Arc::make_mut(&mut game.exile).push(id);
            }
        }
        // Devourer of Destiny revealed from an opening hand: its first-upkeep trigger (from the start's `revealed`, or
        // from the answers of a game recorded without it)
        if let Some(revealed) = &self.start.revealed {
            for (seat, k) in revealed {
                let card = game.players.get(*seat).and_then(|p| p.hand.get(*k)).copied();
                if let Some(card) = card.filter(|c| game.obj(*c).def.name == "Devourer of Destiny") {
                    crate::rules::devourer_first_upkeep(&mut game, card, *seat);
                }
            }
        }
        for (q, a) in self
            .questions
            .iter()
            .zip(self.decisions.iter())
            .take(self.start.decision)
            .filter(|_| self.start.revealed.is_none())
        {
            if q.starts_with("ASK Reveal Devourer of Destiny") && a.bool == Some(true) {
                let card = game.player(a.seat).hand.iter().copied().find(|c| {
                    game.obj(*c).def.name == "Devourer of Destiny" && !game.at_first_upkeep.iter().any(|i| i.card == *c)
                });
                if let Some(card) = card {
                    crate::rules::devourer_first_upkeep(&mut game, card, a.seat);
                }
            }
        }
        // the sideboards, after every card in the game (Java made them with the decks; none of them is listed)
        for (seat, s) in self.start.seats.iter().enumerate() {
            for name in &s.sideboard_cards {
                let def = db.get(name).ok_or_else(|| format!("card not in the engine: {}", name))?;
                let id = game.create_obj(def, seat, Zone::Outside);
                game.player_mut(seat).outside.push(id);
            }
        }
        let header = self.start.fingerprint.lines().nth(1).ok_or("no fingerprint")?;
        // turn <n> step <STEP> active <seat> priority <seat>
        let words: Vec<&str> = header.split(' ').collect();
        game.turn = words.get(1).and_then(|w| w.parse().ok()).ok_or("no turn")?;
        game.step = words.get(3).and_then(|w| Step::parse(w));
        game.active = words.get(5).and_then(|w| w.parse().ok());
        game.priority = words.get(7).and_then(|w| w.parse().ok());
        // the record starts in the first upkeep with its triggers on the stack already
        if game.step == Some(Step::Upkeep) {
            let active = game.active;
            let (mine, rest): (Vec<_>, Vec<_>) =
                std::mem::take(&mut game.at_first_upkeep).into_iter().partition(|i| Some(i.controller) == active);
            game.at_first_upkeep = rest;
            Arc::make_mut(&mut game.stack).extend(mine);
        }
        Ok(game)
    }
}

fn set_printing(game: &mut Game, id: ObjId, printing: Option<&String>) {
    if let Some(p) = printing {
        game.set_printing(id, p);
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
        "MC" => return Some(Answer::ModeCancel),
        _ => {}
    }
    // an ability's mode: `<object>a<ability>m<index>`
    if let Some((obj, mode)) = r.rsplit_once('m') {
        if let (Some((_, a)), Ok(k)) = (obj.rsplit_once('a'), mode.parse::<usize>()) {
            if a.parse::<usize>().is_ok() {
                return Some(Answer::Mode(k));
            }
        }
    }
    // a spell's mode: `<stack object>sm<index>`
    if let Some((_, mode)) = r.rsplit_once("sm") {
        if let Ok(k) = mode.parse::<usize>() {
            return Some(Answer::Mode(k));
        }
    }
    // a double-faced card's face: `<object>f<index>`, the card itself
    if let Some((obj, face)) = r.rsplit_once('f') {
        if let Ok(face) = face.parse::<usize>() {
            let card = resolve(game, obj);
            // an adventure part whose zone Java left behind (the card was bounced from the battlefield): Java finds
            // nothing to play for it, and the click does nothing
            if let Some(Answer::Obj(o)) = card {
                let o = game.obj(o);
                if face == 1 && o.def.adventure.is_some() && o.adventure_zone != o.zone {
                    return Some(Answer::Unknown);
                }
            }
            return card;
        }
    }
    // a split card's (a Room's) half, or its spell: `<object>h<index>`
    if let Some((obj, half)) = r.rsplit_once('h') {
        if let (Some(Answer::Obj(o)), Ok(k)) = (resolve(game, obj), half.parse::<usize>()) {
            return Some(Answer::Ability(o, k));
        }
    }
    // an object's ability: `<object>a<index>`
    if let Some((obj, ability)) = r.rsplit_once('a') {
        if let (Some(Answer::Obj(o)), Ok(i)) = (resolve(game, obj), ability.parse::<usize>()) {
            // a fused split card's abilities: its own spell ability (both halves, `h2`), then each half's
            if game.obj(o).def.fused {
                return Some(Answer::Ability(o, if i == 0 { 2 } else { i - 1 }));
            }
            return Some(match i.checked_sub(game.obj(o).java_ability_offset()) {
                Some(i) => Answer::Ability(o, i),
                None => Answer::Unknown,
            });
        }
    }
    let (kind, rest) = r.split_at(1);
    // a card entering from a hand while its as-enters choices are asked is still in that hand in Java, at its place
    // (Theorist's Sanctum's behold: the hand's cards are shown, the land among them)
    if kind == "H" {
        if let Some(g) = crate::view::as_java(game) {
            return resolve(&g, r);
        }
    }
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
        "O" => {
            let (seat, index) = rest.split_once('.')?;
            let player = game.players.get(seat.parse::<Seat>().ok()?)?;
            player.outside.get(index.parse::<usize>().ok()?).copied().map(Answer::Obj)
        }
        "B" => game.all_permanents().get(rest.parse::<usize>().ok()?).copied().map(Answer::Obj),
        "S" => {
            // Java's stack, with what's resolving still on it
            let index: usize = rest.parse().ok()?;
            let mut stack: Vec<ObjId> = game.stack.iter().map(|i| i.card).collect();
            if let Some((at, card)) = game.resolving {
                stack.insert(at.min(stack.len()), card);
            }
            let n = stack.len();
            if index >= n {
                return None;
            }
            Some(Answer::Obj(stack[n - 1 - index]))
        }
        "E" => exile_order(game).get(rest.parse::<usize>().ok()?).copied().map(Answer::Obj),
        // a waiting trigger: its seat, then its place among that seat's
        "T" => rest.split_once('.').and_then(|(_, i)| i.parse().ok()).map(Answer::Trigger),
        _ => None,
    }
}

/// Exiled cards in the order decision logs name them
pub fn exile_order(game: &Game) -> Vec<crate::state::ObjId> {
    let mut exile = game.exile.to_vec();
    if game.exile_by_creation {
        // start cards in their order, then prepared copies in their permanents' (`DecisionLog.visit`), then the rest
        exile.sort_by_key(|o| match game.obj(*o).prepared_for {
            Some((p, _)) => (1, p.0),
            None if game.obj(*o).token => (2, o.0),
            None => (0, o.0),
        });
    }
    exile
}

/// Where an object is, as `DecisionLog.ref` writes it.
pub fn reference(game: &Game, id: ObjId) -> String {
    // a hand card while one enters from a hand: its place in Java's hand, which still holds the entering card (as
    // `resolve` reads `H` refs)
    if game.obj(id).zone == Zone::Hand {
        if let Some(g) = crate::view::as_java(game) {
            return reference(&g, id);
        }
    }
    for (s, p) in game.players.iter().enumerate() {
        for (prefix, zone) in [("H", &p.hand), ("L", &p.library), ("G", &p.graveyard)] {
            if let Some(i) = zone.iter().position(|x| *x == id) {
                return format!("{}{}.{}", prefix, s, i);
            }
        }
    }
    if let Some(i) = game.all_permanents().iter().position(|x| *x == id) {
        return format!("B{}", i);
    }
    if let Some(i) = game.stack.iter().rev().position(|x| x.card == id) {
        return format!("S{}", i);
    }
    if let Some(i) = exile_order(game).iter().position(|x| *x == id) {
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
    /// called as a recorded view's question is asked: the game, the recorded view
    pub on_view: Option<OnView<'a>>,
    /// called as a recorded prompt's question is asked
    pub on_prompt: Option<OnPrompt<'a>>,
    prompt_at: usize,
    /// the engine's log lines (`Game::log`), each with the answers given before it
    pub log: Vec<(usize, String)>,
    /// the first trace entry that matched with the random numbers already elsewhere: what drew one Java didn't (or
    /// the other way round) happened before it
    pub random_split: Option<usize>,
}

impl<'a> ReplayDecider<'a> {
    pub fn new(shadow: &'a ShadowGame) -> Self {
        ReplayDecider {
            shadow,
            next: shadow.start.decision,
            trace: Vec::new(),
            failure: None,
            on_view: None,
            on_prompt: None,
            prompt_at: 0,
            log: Vec::new(),
            random_split: None,
        }
    }

    /// The engine's new log lines, after the answers given so far
    pub fn take_log(&mut self, game: &Game) {
        let seen: usize = self.log.len();
        if let Some(lines) = &game.log {
            for l in lines.iter().skip(seen) {
                self.log.push((self.next, l.clone()));
            }
        }
    }

    fn fail(&mut self, why: String) {
        if self.failure.is_none() {
            let split = match self.random_split {
                Some(i) => format!(
                    "\n(the random numbers parted before trace entry {}, after {:?} answers)",
                    i,
                    self.shadow.trace.get(i).and_then(|e| e.decision)
                ),
                None => String::new(),
            };
            self.failure = Some(why + &split);
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
        self.take_log(game);
        if let Some(f) = self.on_prompt.as_mut() {
            let prompts = &self.shadow.prompts;
            while self.prompt_at < prompts.len() && prompts[self.prompt_at].decision < self.next {
                self.prompt_at += 1;
            }
            if let Some(p) = prompts.get(self.prompt_at).filter(|p| p.decision == self.next && p.seat == seat) {
                f(game, query, p);
            }
        }
        if d.seat == seat {
            if let Some(f) = self.on_view.as_mut() {
                for v in self.shadow.views.iter().filter(|v| v.decision == self.next) {
                    f(game, v);
                }
            }
        }
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
                    self.fail(format!(
                        "answer {} names {}, which isn't in this game ({}; asked {:?})",
                        self.next, r, asked, query
                    ));
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
            if std::env::var_os("PLAYMAT_SHADOW_BF").is_some() {
                let bf: Vec<String> = game
                    .all_permanents()
                    .iter()
                    .enumerate()
                    .map(|(i, o)| {
                        format!(
                            "B{}={}:{}{}",
                            i,
                            o.0,
                            game.obj(*o).name(),
                            if game.obj(*o).until_end_of_turn_def.is_some() { "*" } else { "" }
                        )
                    })
                    .collect();
                eprintln!("  {}", bf.join(" "));
                let exile: Vec<String> = game
                    .exile
                    .iter()
                    .enumerate()
                    .map(|(i, o)| format!("E{}={}:{}{:?}", i, o.0, game.obj(*o).name(), game.obj(*o).prepared_for))
                    .collect();
                eprintln!("  {}", exile.join(" "));
                for (seat, p) in game.players.iter().enumerate() {
                    let hand: Vec<String> = p
                        .hand
                        .iter()
                        .enumerate()
                        .map(|(i, o)| format!("H{}.{}={}", seat, i, game.obj(*o).name()))
                        .collect();
                    eprintln!("  {}", hand.join(" "));
                }
            }
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
            Some(e) if e.fp == hash => {
                if self.random_split.is_none() && e.random.is_some_and(|r| r != game.rng.state()) {
                    self.random_split = Some(i);
                }
                self.trace.push(hash)
            }
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
    replay_with_views(shadow, db, None)
}

/// Replays a shadow game, calling `on_view` at each recorded view's question (docs/rust/Views.md).
pub fn replay_with_views<'a>(shadow: &'a ShadowGame, db: &CardDb, on_view: Option<OnView<'a>>) -> ReplayReport {
    replay_checked(shadow, db, on_view, None).0
}

/// Replays a shadow game with its views and prompts compared (`on_view`, `on_prompt`) and the engine's game log
/// kept: the report, and the log's lines, each with the answers given before it.
pub fn replay_checked<'a>(
    shadow: &'a ShadowGame,
    db: &CardDb,
    on_view: Option<OnView<'a>>,
    on_prompt: Option<OnPrompt<'a>>,
) -> (ReplayReport, Vec<(usize, String)>) {
    let mut report = ReplayReport { entries: shadow.trace.len(), ..ReplayReport::default() };
    let mut game = match shadow.start_game(db) {
        Ok(g) => g,
        Err(e) => {
            report.failure = Some(e);
            return (report, Vec::new());
        }
    };
    if !shadow.log.is_empty() {
        game.log = Some(Vec::new());
    }
    let mut decider = ReplayDecider::new(shadow);
    decider.on_view = on_view;
    decider.on_prompt = on_prompt;
    crate::rules::play_from_priority(&mut game, &mut decider, shadow.turns);
    decider.take_log(&game);
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
    (report, decider.log)
}
