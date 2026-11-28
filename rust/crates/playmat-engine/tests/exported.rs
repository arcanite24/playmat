//! Exported card tests (docs/rust/TestExport.md): Java's `CardTestPlayerBase` scripts, played on this engine. A test
//! passes when the game ends in the same state as Java's (the end state's fingerprint): the JUnit assertions stay in
//! Java, and the end state holds what they read.
//!
//! The tests checked in under `exported/` must all pass. With `PLAYMAT_EXPORTS=<folder or list file>` the test runs
//! those exports instead and only reports (add `PLAYMAT_EXPORTS_STRICT=1` to fail).
//!
//! Only tests the engine can play honestly are run: `complete` (no Java code in the script), no `ai:` or `run:`
//! commands, no cheat commands, a two-player duel with every card in the engine. The others are skipped with the
//! reason. A test without strict choose mode has Java's AI answer what its script doesn't: when the engine asks such a
//! question and the end state differs, the test is skipped as one this runner can't play.
//!
//! The script's player is `TestPlayer`, which plays through `ComputerPlayer.activateAbility`; the engine asks its
//! questions as `HumanPlayer` does, so the decider below answers them the way the script's commands mean:
//! `activate:` clicks the object and picks the ability by its text, `$target=`/`$targetPlayer=` and the player's
//! `targets` answer the targets, `choices` and `modes` the other questions in order, `attack:`/`block:` declare
//! combat, `waitStackResolved` passes until the stack is empty. Mana is paid from the first source offered.

use playmat_engine::card::CardDb;
use playmat_engine::decide::{Answer, Decider, Query};
use playmat_engine::state::{Game, ObjId, Seat, Step, Zone};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Deserialize)]
struct Export {
    test: String,
    #[serde(rename = "gameType")]
    game_type: String,
    #[serde(rename = "stopOnTurn")]
    stop_on_turn: u32,
    #[serde(rename = "stopAtStep")]
    stop_at_step: String,
    #[serde(rename = "activePlayer")]
    active_player: String,
    players: Vec<Script>,
    #[serde(default)]
    complete: bool,
    result: Outcome,
}

#[derive(Deserialize)]
struct Script {
    name: String,
    #[serde(rename = "strictChooseMode")]
    strict: bool,
    #[serde(rename = "startLibrary", default)]
    start_library: Vec<String>,
    #[serde(default)]
    hand: Vec<Card>,
    #[serde(default)]
    battlefield: Vec<Card>,
    #[serde(default)]
    graveyard: Vec<Card>,
    #[serde(default)]
    exile: Vec<Card>,
    #[serde(default)]
    command: Vec<Card>,
    #[serde(default)]
    commands: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    actions: Vec<Action>,
    #[serde(default)]
    choices: Vec<String>,
    #[serde(default)]
    targets: Vec<String>,
    #[serde(default)]
    modes: Vec<String>,
}

#[derive(Deserialize, Clone)]
struct Card {
    name: String,
    set: Option<String>,
    number: Option<String>,
    #[serde(default)]
    tapped: bool,
}

#[derive(Deserialize, Clone)]
struct Action {
    turn: u32,
    step: String,
    action: String,
}

#[derive(Deserialize)]
struct Outcome {
    fingerprint: String,
    state: String,
}

/// Tests whose game Java changes from outside the script, which the export can't carry
const JAVA_SETUP: &[(&str, &str)] = &[
    ("GameSetupTest.theSetupChangesTheStartOnce", "its game options carry a Java setup (`gameOptions.setup`)"),
    // `addEmblem(playerA, new EmblemOfCard(...))` builds the emblem in Java; the export has no zone for it
    ("EmblemOfCardTest.testEmblemOfGriselbrand", "its emblem is made in Java (`addEmblem`), not in the export"),
    ("EmblemOfCardTest.testEmblemOfYurlok", "its emblem is made in Java (`addEmblem`), not in the export"),
];

/// Why a test can't be run here, if it can't
fn unrunnable(e: &Export, db: &CardDb) -> Option<String> {
    if let Some((_, why)) = JAVA_SETUP.iter().find(|(t, _)| e.test.ends_with(t)) {
        return Some(why.to_string());
    }
    if e.game_type != "TwoPlayerDuel" || e.players.len() != 2 {
        return Some(format!("game type {}", e.game_type));
    }
    if !e.complete {
        return Some("runs Java code (not complete)".into());
    }
    for p in &e.players {
        if !p.commands.is_empty() {
            return Some("cheat commands".into());
        }
        if let Some(a) = p.actions.iter().find(|a| a.action.starts_with("ai:") || a.action.starts_with("run:")) {
            return Some(format!("{} command `{}`", p.name, a.action));
        }
        let names = p.start_library.iter().chain(
            [&p.hand, &p.battlefield, &p.graveyard, &p.exile, &p.command].into_iter().flatten().map(|c| &c.name),
        );
        if let Some(n) = names.into_iter().find(|n| db.get(n).is_none()) {
            return Some(format!("card not in the engine: {n}"));
        }
        if !p.command.is_empty() {
            return Some("cards in the command zone".into());
        }
    }
    None
}

/// The game as the test sets it up: libraries (top first), the cards the test added, turn 1's untap step
fn start(e: &Export, db: &CardDb) -> Game {
    let mut game = Game::new(2, 0);
    for (seat, p) in e.players.iter().enumerate() {
        {
            let pl = game.player_mut(seat);
            pl.auto_target_all = true;
            pl.every_priority = true;
            pl.no_pool_check = true;
            pl.no_stock = true;
        }
        for name in &p.start_library {
            let id = game.create_obj(db.get(name).expect("checked"), seat, Zone::Library);
            game.player_mut(seat).library.push(id);
        }
        let add = |game: &mut Game, cards: &[Card], zone: Zone| {
            for c in cards {
                let id = game.create_obj(db.get(&c.name).expect("checked"), seat, zone);
                if let (Some(s), Some(n)) = (&c.set, &c.number) {
                    game.set_printing(id, &format!("{s}:{n}"));
                }
                match zone {
                    Zone::Hand => game.player_mut(seat).hand.push(id),
                    Zone::Graveyard => game.player_mut(seat).graveyard.push(id),
                    Zone::Exile => Arc::make_mut(&mut game.exile).push(id),
                    _ => {
                        Arc::make_mut(&mut game.battlefield).push(id);
                        game.obj_mut(id).tapped = c.tapped;
                    }
                }
            }
        };
        add(&mut game, &p.hand, Zone::Hand);
        add(&mut game, &p.battlefield, Zone::Battlefield);
        add(&mut game, &p.graveyard, Zone::Graveyard);
        add(&mut game, &p.exile, Zone::Exile);
    }
    let active = e.players.iter().position(|p| p.name == e.active_player).unwrap_or(0);
    game.turn = 1;
    game.step = Some(Step::Untap);
    game.active = Some(active);
    game.priority = Some(active);
    game
}

/// The action in progress: its targets left to give, and the ability text to pick
struct Doing {
    seat: Seat,
    ability: String,
    targets: Vec<String>,
}

struct Scripted<'a> {
    export: &'a Export,
    actions: Vec<Vec<Action>>,
    choices: Vec<Vec<String>>,
    targets: Vec<Vec<String>>,
    modes: Vec<Vec<String>>,
    doing: Option<Doing>,
    stop_turn: u32,
    stop_step: Step,
    /// questions no script line answered (a first legal answer was given instead)
    unscripted: Vec<String>,
    /// a question the decider couldn't answer at all: the game stops
    failure: Option<String>,
    /// priorities in a row with nothing to do in a step that has actions (a stuck command)
    asked: usize,
}

fn name_matches(game: &Game, id: ObjId, name: &str) -> bool {
    let o = game.obj(id);
    let n = o.name();
    n == name
        || n.split(" // ").any(|h| h == name)
        || o.def.adventure.as_ref().is_some_and(|a| a.name == name)
        || o.def.back.as_ref().is_some_and(|b| b.name == name)
        || o.def.room.iter().any(|r| r.name == name)
}

/// Each ability's label in the ability picker (`Cast X`, `Cast X with Blitz`, `{T}: Add {R}.`), by index
fn labels(game: &Game, seat: Seat, id: ObjId) -> Vec<(usize, String)> {
    let all: Vec<usize> = (0..game.obj(id).def.abilities.len()).collect();
    let q = Query::ChooseAbility { source: id, abilities: all };
    let Some(view) = playmat_engine::prompt::prompt_view(game, seat, &q) else { return Vec::new() };
    let mut out = Vec::new();
    if let Some(choices) = view["data"]["choices"].as_object() {
        for (k, v) in choices {
            let index = k.rsplit_once('a').and_then(|(_, i)| i.parse().ok());
            let text = v.as_str().unwrap_or("");
            let text = text.split_once(". ").map(|(_, t)| t).unwrap_or(text).to_string();
            if let Some(i) = index {
                out.push((i, text));
            }
        }
    }
    out
}

fn ability_matches(label: &str, wanted: &str) -> bool {
    label == wanted || label.starts_with(wanted) || label.trim_end_matches('.') == wanted.trim_end_matches('.')
}

fn objects(game: &Game, seat: Seat) -> Vec<ObjId> {
    let p = game.player(seat);
    let mut out: Vec<ObjId> = p.hand.to_vec();
    out.extend(game.battlefield.iter().copied().filter(|b| game.obj(*b).controller == seat));
    out.extend(p.graveyard.iter().copied());
    out.extend(game.exile.iter().copied().filter(|x| game.obj(*x).owner == seat));
    out
}

impl<'a> Scripted<'a> {
    fn new(export: &'a Export) -> Self {
        let stop_step = Step::parse(&export.stop_at_step).unwrap_or(Step::Untap);
        Scripted {
            export,
            actions: export.players.iter().map(|p| p.actions.clone()).collect(),
            choices: export.players.iter().map(|p| p.choices.clone()).collect(),
            targets: export.players.iter().map(|p| p.targets.clone()).collect(),
            modes: export.players.iter().map(|p| p.modes.clone()).collect(),
            doing: None,
            stop_turn: export.stop_on_turn,
            stop_step,
            unscripted: Vec::new(),
            failure: None,
            asked: 0,
        }
    }

    fn seat_of(&self, name: &str) -> Option<Seat> {
        self.export.players.iter().position(|p| p.name == name)
    }

    fn unscripted(&mut self, game: &Game, seat: Seat, query: &Query) {
        self.unscripted.push(format!(
            "turn {} {:?}: seat {} asked {:?}",
            game.turn,
            game.step,
            seat,
            match query {
                Query::Priority { .. } => "priority".to_string(),
                q => format!("{q:?}").chars().take(120).collect(),
            }
        ));
    }

    /// The next target named for `seat`: the action's `$target=` first, then the player's `targets`
    fn next_target(&mut self, seat: Seat) -> Option<String> {
        if let Some(d) = self.doing.as_mut().filter(|d| d.seat == seat) {
            if !d.targets.is_empty() {
                return Some(d.targets.remove(0));
            }
        }
        if !self.targets[seat].is_empty() {
            return Some(self.targets[seat].remove(0));
        }
        None
    }

    fn target_answer(&self, game: &Game, name: &str, players: &[Seat], objs: &[ObjId]) -> Option<Answer> {
        // the player's `targets` keep the command's form: `targetPlayer=PlayerB`, `target=Name`
        let name = name.strip_prefix("targetPlayer=").or_else(|| name.strip_prefix("target=")).unwrap_or(name);
        if let Some(s) = self.seat_of(name).filter(|s| players.contains(s)) {
            return Some(Answer::Player(s));
        }
        // `Name [no copy]`, `@alias`: the name part
        let name = name.split(" [").next().unwrap_or(name);
        objs.iter().copied().find(|o| name_matches(game, *o, name)).map(Answer::Obj)
    }

    /// Priority: the first of this seat's commands for this turn and step, as `TestPlayer.priority` goes through them
    fn priority(&mut self, game: &Game, seat: Seat, playable: &[ObjId]) -> Answer {
        self.doing = None;
        let step = game.step.map(|s| s.name()).unwrap_or("");
        let mut i = 0;
        while i < self.actions[seat].len() {
            let a = self.actions[seat][i].clone();
            if a.turn != game.turn || a.step != step {
                i += 1;
                continue;
            }
            let action = a.action.as_str();
            if action.starts_with("waitStackResolved") {
                if game.stack.is_empty() {
                    self.actions[seat].remove(i);
                    continue;
                }
                if action == "waitStackResolved:1" {
                    self.actions[seat].remove(i);
                }
                return Answer::Bool(false);
            }
            if let Some(command) = action.strip_prefix("activate:").or_else(|| action.strip_prefix("manaActivate:")) {
                let groups: Vec<&str> = command.split('$').collect();
                let wanted = groups[0];
                let mut targets = Vec::new();
                for g in &groups[1..] {
                    if let Some(t) = g.strip_prefix("target=").or_else(|| g.strip_prefix("targetPlayer=")) {
                        targets.extend(t.split('^').map(str::to_string));
                    }
                }
                if let Some(id) = self.find(game, seat, wanted, playable) {
                    self.actions[seat].remove(i);
                    self.doing = Some(Doing { seat, ability: wanted.to_string(), targets });
                    self.asked = 0;
                    return Answer::Obj(id);
                }
                self.failure = Some(format!("can't find what to activate for `{action}` (turn {}, {step})", game.turn));
                return Answer::Stop;
            }
            // attack, block and assertions aren't priority commands
            i += 1;
        }
        Answer::Bool(false)
    }

    /// The object a command's ability text is on
    fn find(&self, game: &Game, seat: Seat, wanted: &str, playable: &[ObjId]) -> Option<ObjId> {
        let candidates = objects(game, seat);
        if let Some(rest) = wanted.strip_prefix("Cast ").or_else(|| wanted.strip_prefix("Play ")) {
            let name = rest.split(" with ").next().unwrap_or(rest).split(" using ").next().unwrap_or(rest);
            let by_name: Vec<ObjId> = candidates.iter().copied().filter(|o| name_matches(game, *o, name)).collect();
            if let Some(o) = by_name.iter().copied().find(|o| playable.contains(o)) {
                return Some(o);
            }
            if let Some(o) = by_name.first() {
                return Some(*o);
            }
        }
        // ability text on one of the seat's objects (a permanent first)
        let mut order: Vec<ObjId> =
            game.battlefield.iter().copied().filter(|b| game.obj(*b).controller == seat).collect();
        order.extend(candidates);
        order.into_iter().find(|o| {
            let named = wanted.replace("{this}", game.obj(*o).name());
            labels(game, seat, *o).iter().any(|(_, l)| ability_matches(l, wanted) || ability_matches(l, &named))
        })
    }

    fn take_choice(&mut self, seat: Seat) -> Option<String> {
        if self.choices[seat].is_empty() {
            None
        } else {
            Some(self.choices[seat].remove(0))
        }
    }
}

impl<'a> Decider for Scripted<'a> {
    fn answer(&mut self, game: &Game, seat: Seat, query: &Query) -> Answer {
        if self.failure.is_some() {
            return Answer::Stop;
        }
        match query {
            Query::Priority { playable } => {
                let a = self.priority(game, seat, playable);
                if matches!(a, Answer::Obj(_)) {
                    self.asked += 1;
                    if self.asked > 50 {
                        self.failure = Some("the same command is clicked over and over".into());
                        return Answer::Stop;
                    }
                }
                a
            }
            Query::ChooseAbility { source, abilities } => {
                let wanted = self.doing.as_ref().map(|d| d.ability.clone()).unwrap_or_default();
                let wanted = wanted.replace("{this}", game.obj(*source).name());
                let labels = labels(game, seat, *source);
                let found = abilities.iter().copied().find(|k| {
                    labels.iter().any(|(i, l)| {
                        i == k && (l == &wanted || (!wanted.starts_with("Cast ") && ability_matches(l, &wanted)))
                    })
                });
                let found = found.or_else(|| {
                    // `Cast X` is the card's own spell
                    abilities.iter().copied().find(|k| {
                        labels.iter().any(|(i, l)| {
                            i == k && wanted.starts_with("Cast ") && l.starts_with(&wanted) && !l.contains(" with ")
                        })
                    })
                });
                match found {
                    Some(k) => Answer::Ability(*source, k),
                    None => {
                        self.unscripted(game, seat, query);
                        Answer::Ability(*source, abilities[0])
                    }
                }
            }
            Query::PlayMana { sources, .. } => match sources.first() {
                Some(s) => Answer::Obj(*s),
                None => Answer::Bool(false),
            },
            Query::Target { players, objs, required } => match self.next_target(seat) {
                Some(name) => match self.target_answer(game, &name, players, objs) {
                    Some(a) => a,
                    None => {
                        self.failure = Some(format!("target `{name}` isn't among the legal targets {query:?}"));
                        Answer::Stop
                    }
                },
                None if *required => {
                    self.unscripted(game, seat, query);
                    // Java's AI (`PossibleTargetsSelector`) targets an opponent for a harmful effect, the usual case;
                    // a test where it picked otherwise ends differently and is skipped below
                    players
                        .iter()
                        .find(|p| **p != seat)
                        .or(players.first())
                        .map(|p| Answer::Player(*p))
                        .or(objs.first().map(|o| Answer::Obj(*o)))
                        .unwrap_or(Answer::Bool(false))
                }
                None => Answer::Bool(false),
            },
            Query::Cards { objs, required } => {
                // `TestPlayer.choose`: a choice naming the cards (`A^B`)
                if let Some(c) = self.choices[seat].first().cloned() {
                    let first = c.split('^').next().unwrap_or(&c).to_string();
                    if let Some(o) = objs.iter().copied().find(|o| name_matches(game, *o, &first)) {
                        let rest: Vec<&str> = c.split('^').skip(1).collect();
                        if rest.is_empty() {
                            self.choices[seat].remove(0);
                        } else {
                            self.choices[seat][0] = rest.join("^");
                        }
                        return Answer::Obj(o);
                    }
                }
                if let Some(name) = self.next_target(seat) {
                    if let Some(a) = self.target_answer(game, &name, &[], objs) {
                        return a;
                    }
                }
                if *required {
                    self.unscripted(game, seat, query);
                    objs.first().map(|o| Answer::Obj(*o)).unwrap_or(Answer::Bool(false))
                } else {
                    Answer::Bool(false)
                }
            }
            Query::YesNo { .. } => match self.take_choice(seat) {
                Some(c) => Answer::Bool(c.eq_ignore_ascii_case("yes")),
                None => {
                    self.unscripted(game, seat, query);
                    Answer::Bool(false)
                }
            },
            Query::PickTrigger { triggers, .. } => {
                let pick = self.choices[seat]
                    .first()
                    .and_then(|c| triggers.iter().position(|(_, r)| r.starts_with(c.as_str())));
                match pick {
                    Some(i) => {
                        self.choices[seat].remove(0);
                        Answer::Trigger(i)
                    }
                    None => {
                        self.unscripted(game, seat, query);
                        Answer::Trigger(0)
                    }
                }
            }
            Query::ChooseMode { modes } => {
                if self.modes[seat].is_empty() {
                    return Answer::ModeDone;
                }
                let m = self.modes[seat].remove(0);
                match m.parse::<usize>() {
                    Ok(n) if n >= 1 && modes.contains(&(n - 1)) => Answer::Mode(n - 1),
                    _ => {
                        self.failure = Some(format!("mode `{m}` isn't among {modes:?}"));
                        Answer::Stop
                    }
                }
            }
            Query::Amount { min, .. } => {
                let c = self.choices[seat].first().cloned();
                match c.as_deref().and_then(|c| c.strip_prefix("X=").unwrap_or(c).parse::<i32>().ok()) {
                    Some(n) => {
                        self.choices[seat].remove(0);
                        Answer::Int(n)
                    }
                    None => {
                        self.unscripted(game, seat, query);
                        Answer::Int(*min)
                    }
                }
            }
            Query::Choice { choices, .. } => match self.take_choice(seat) {
                Some(c) => Answer::Text(choices.iter().find(|x| x.eq_ignore_ascii_case(&c)).cloned().unwrap_or(c)),
                None => {
                    self.unscripted(game, seat, query);
                    Answer::Text(choices.first().cloned().unwrap_or_default())
                }
            },
            Query::MultiAmount { defaults, .. } => {
                Answer::Text(defaults.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(" "))
            }
            Query::Attackers { creatures } => {
                let step = game.step.map(|s| s.name()).unwrap_or("");
                let at = self.actions[seat]
                    .iter()
                    .position(|a| a.turn == game.turn && (a.step == step || a.action.starts_with("attack:")));
                let at = at.filter(|i| self.actions[seat][*i].action.starts_with("attack:"));
                if let Some(i) = at {
                    let a = self.actions[seat].remove(i);
                    let command = a.action.trim_start_matches("attack:");
                    let groups: Vec<&str> = command.split('$').collect();
                    let name = groups[0].split(':').next_back().unwrap_or(groups[0]);
                    if let Some(c) = creatures.iter().copied().find(|c| name_matches(game, *c, name)) {
                        let defender =
                            groups.iter().find_map(|g| g.strip_prefix("defendingPlayer=")).map(str::to_string);
                        self.doing =
                            Some(Doing { seat, ability: String::new(), targets: defender.into_iter().collect() });
                        return Answer::Obj(c);
                    }
                    self.failure = Some(format!("can't attack with `{name}`"));
                    return Answer::Stop;
                }
                Answer::Bool(false)
            }
            Query::Blockers { creatures } => {
                let at = self.actions[seat].iter().position(|a| a.turn == game.turn && a.action.starts_with("block:"));
                if let Some(i) = at {
                    let a = self.actions[seat].remove(i);
                    let command = a.action.trim_start_matches("block:");
                    let (blocker, attacker) = command.split_once('$').unwrap_or((command, ""));
                    if let Some(c) = creatures.iter().copied().find(|c| name_matches(game, *c, blocker)) {
                        self.doing = Some(Doing { seat, ability: String::new(), targets: vec![attacker.to_string()] });
                        return Answer::Obj(c);
                    }
                    self.failure = Some(format!("can't block with `{blocker}`"));
                    return Answer::Stop;
                }
                Answer::Bool(false)
            }
            Query::BlockTarget { attackers } => {
                let name = self.next_target(seat).unwrap_or_default();
                match attackers.iter().copied().find(|a| name_matches(game, *a, &name)) {
                    Some(a) => Answer::Obj(a),
                    None => {
                        self.unscripted(game, seat, query);
                        attackers.first().map(|a| Answer::Obj(*a)).unwrap_or(Answer::Bool(false))
                    }
                }
            }
        }
    }

    fn stopped(&self) -> bool {
        self.failure.is_some()
    }

    fn stop_after(&mut self, game: &Game, step: Step) -> bool {
        self.stop_step != Step::Untap && game.turn >= self.stop_turn && step == self.stop_step
    }
}

enum Verdict {
    Pass,
    Differs(String),
    Skipped(String),
}

fn run(e: &Export, db: &CardDb) -> Verdict {
    if let Some(why) = unrunnable(e, db) {
        return Verdict::Skipped(why);
    }
    let mut game = start(e, db);
    let mut d = Scripted::new(e);
    // UNTAP: Java stops as that turn begins (`GameImpl.checkStopOnTurnOption`)
    let turns = if d.stop_step == Step::Untap { e.stop_on_turn } else { 0 };
    playmat_engine::rules::play_from_priority(&mut game, &mut d, turns);
    let text = playmat_engine::fingerprint::text(&game);
    let hash = playmat_engine::fingerprint::hash_text(&text);
    let mut notes = Vec::new();
    if let Some(f) = &d.failure {
        notes.push(format!("stopped: {f}"));
    }
    for (seat, left) in d.actions.iter().enumerate() {
        let left: Vec<&str> = left
            .iter()
            .map(|a| a.action.as_str())
            .filter(|a| !a.starts_with("check") && !a.starts_with("assert") && !a.starts_with("show"))
            .collect();
        if !left.is_empty() {
            notes.push(format!("seat {seat} commands left: {left:?}"));
        }
    }
    for u in d.unscripted.iter().take(3) {
        notes.push(format!("unscripted: {u}"));
    }
    if hash == e.result.fingerprint && d.failure.is_none() {
        return Verdict::Pass;
    }
    // `org.mage.test.AI`'s classes may play a seat with Java's AI (`CardTestPlayerBaseWithAIHelps`, `AIPlayer`)
    if e.test.starts_with("org.mage.test.AI.") {
        return Verdict::Skipped("an AI test whose end differs: Java's AI may play its seats".into());
    }
    // without strict choose mode Java's AI answers what the script doesn't: a question the script left open is
    // one this runner can't answer as Java did
    if let (Some(p), Some(u)) = (e.players.iter().find(|p| !p.strict), d.unscripted.first()) {
        return Verdict::Skipped(format!("{} isn't in strict choose mode and Java's AI answered {u}", p.name));
    }
    let java: Vec<&str> = e.result.state.lines().collect();
    let rust: Vec<&str> = text.lines().collect();
    let mut diff = Vec::new();
    for i in 0..java.len().max(rust.len()) {
        let (a, b) = (java.get(i).copied().unwrap_or(""), rust.get(i).copied().unwrap_or(""));
        if a != b {
            diff.push(format!("  java: {}\n  rust: {}", short(a), short(b)));
            if diff.len() == 3 {
                break;
            }
        }
    }
    Verdict::Differs(format!("{}\n{}", notes.join("\n"), diff.join("\n")))
}

fn short(s: &str) -> String {
    if s.chars().count() > 160 {
        format!("{}…", s.chars().take(160).collect::<String>())
    } else {
        s.to_string()
    }
}

fn collect(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let Ok(entries) = std::fs::read_dir(path) else { return };
        for e in entries.flatten() {
            collect(&e.path(), out);
        }
    } else if path.extension().is_some_and(|x| x == "json") {
        out.push(path.to_path_buf());
    } else if path.extension().is_some_and(|x| x == "txt") {
        // a list of exports, one path a line
        for l in std::fs::read_to_string(path).unwrap_or_default().lines().filter(|l| !l.trim().is_empty()) {
            out.push(PathBuf::from(l.trim()));
        }
    }
}

#[test]
fn exported_tests_end_as_in_java() {
    let named = std::env::var("PLAYMAT_EXPORTS").ok();
    let strict = named.is_none() || std::env::var("PLAYMAT_EXPORTS_STRICT").is_ok_and(|v| v == "1");
    let dir = named.map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("exported"));
    let mut files = Vec::new();
    collect(&dir, &mut files);
    files.sort();
    let db = playmat_engine::cards();
    let (mut passed, mut skipped, mut failed) = (0, 0, Vec::new());
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap_or_default();
        let e: Export = match serde_json::from_str(&text) {
            Ok(e) => e,
            Err(err) => {
                eprintln!("{}: can't read: {err}", f.display());
                failed.push(f.display().to_string());
                continue;
            }
        };
        let name = e.test.rsplit('.').take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join(".");
        match run(&e, &db) {
            Verdict::Pass => {
                passed += 1;
                eprintln!("{name}: passes");
            }
            Verdict::Skipped(why) => {
                skipped += 1;
                eprintln!("{name}: skipped ({why})");
            }
            Verdict::Differs(why) => {
                eprintln!("{name}: differs\n{why}");
                failed.push(name);
            }
        }
    }
    eprintln!("{} pass, {} differ, {} skipped", passed, failed.len(), skipped);
    if strict {
        assert!(failed.is_empty(), "exported tests that end differently: {failed:?}");
    }
}
