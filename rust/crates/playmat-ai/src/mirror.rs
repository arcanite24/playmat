//! A Rust copy of a game Java hosts, kept in step answer by answer, so the Rust AI can play a seat of it
//! (docs/rust/Ai.md). Java is the game; this side follows.
//!
//! Java starts the copy from the game at its first priority (the same start as a shadow game), then, each time it
//! asks the AI's seat a question, sends what happened since: the other seat's answers (`DecisionLog` references) and
//! its trace (state fingerprints). The copy plays those answers, checks its own fingerprints against Java's, and when
//! it reaches the same question for the AI's seat, searches and sends back the answer as a `DecisionLog` reference.
//! Any difference (a fingerprint, who is asked, an answer that doesn't fit) ends the copy: Java plays on without it.
//!
//! The messages are JSON lines. Java sends `start` (once), `decision`, `trace`, `ask` and `end`; this side answers
//! `start` with `ready` (or `error`) and each `ask` with `answer` or `diverged`.

use crate::search::{Config, Table};
use playmat_engine::card::CardDb;
use playmat_engine::decide::{Answer, Decider, Query};
use playmat_engine::fingerprint;
use playmat_engine::replay::{reference, resolve, ShadowDecision, ShadowDeck, ShadowGame, ShadowStart};
use playmat_engine::rules;
use playmat_engine::state::{Game, Seat, Zone};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::io::{BufRead, Write};

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Message {
    Start {
        seat: Seat,
        start: ShadowStart,
        #[serde(default)]
        config: Option<ConfigJson>,
        /// the seat auto-chooses at level 2, as Java's AI players do (a human's default is 1)
        #[serde(default, rename = "autoTargetAll")]
        auto_target_all: bool,
        /// the other seat is asked at every priority (Java logs the passes its settings make)
        #[serde(default, rename = "everyPriority")]
        every_priority: bool,
        /// the other seat's `confirmEmptyManaPool`
        #[serde(default, rename = "poolCheck")]
        pool_check: Option<bool>,
    },
    Decision {
        #[serde(flatten)]
        decision: ShadowDecision,
    },
    Trace {
        fp: String,
        /// the fingerprint's text (PLAYMAT_AI_DEBUG), to show a difference
        #[serde(default)]
        text: Option<String>,
    },
    Ask {
        #[serde(default)]
        question: String,
        /// Java's own answer, to follow instead of searching (replaying a recorded game through the copy)
        #[serde(default)]
        follow: Option<ShadowDecision>,
    },
    End,
}

#[derive(Deserialize, Default)]
struct ConfigJson {
    budget: Option<u64>,
    width: Option<usize>,
    #[serde(rename = "extraTurns")]
    extra_turns: Option<u32>,
}

/// Runs one game: reads Java's messages from `input`, writes answers to `out`, until Java ends the game, the copy
/// differs, or the input closes.
pub fn run(db: &CardDb, input: impl BufRead, mut out: impl Write) -> Result<(), String> {
    let mut lines = input.lines();
    let first = lines.next().ok_or("no start message")?.map_err(|e| e.to_string())?;
    let (seat, start, cfg, auto_all, every, pool_check) = match serde_json::from_str::<Message>(&first) {
        Ok(Message::Start { seat, start, config, auto_target_all, every_priority, pool_check }) => {
            (seat, start, config.unwrap_or_default(), auto_target_all, every_priority, pool_check)
        }
        Ok(_) => return Err("the first message must be start".into()),
        Err(e) => return Err(format!("bad start message: {e}")),
    };
    let shadow = ShadowGame {
        version: 1,
        seed: 0,
        decks: Vec::<ShadowDeck>::new(),
        start,
        decisions: Vec::new(),
        questions: Vec::new(),
        trace: Vec::new(),
        result: String::new(),
        turns: 0,
    };
    let mut game = match shadow.start_game(db) {
        Ok(g) => g,
        Err(e) => {
            send(&mut out, json!({"type": "error", "message": e}));
            return Err(e);
        }
    };
    game.player_mut(seat).auto_target_all = auto_all;
    // the Java AI's seat (auto-targets at level 2): its pool always pays by itself
    game.player_mut(seat).no_stock = auto_all;
    let other = game.player_mut(game.opponent(seat));
    other.every_priority = every;
    other.no_pool_check = pool_check == Some(false);
    send(&mut out, json!({"type": "ready"}));
    let d = Config::default();
    let config = Config {
        budget: cfg.budget.unwrap_or(d.budget),
        width: cfg.width.unwrap_or(d.width),
        extra_turns: cfg.extra_turns.unwrap_or(d.extra_turns),
        ..d
    };
    let mut ai = vec![None, None];
    ai[seat] = Some(config);
    let mut mirror = Mirror {
        lines: Box::new(lines),
        out: Box::new(out),
        bot: seat,
        table: Table::new(ai),
        java: VecDeque::new(),
        rust: VecDeque::new(),
        last: None,
        texts: VecDeque::new(),
        rust_texts: VecDeque::new(),
        diverged: None,
        differs: None,
        ended: false,
        asked: 0,
        waiting: false,
    };
    rules::play_from_priority(&mut game, &mut mirror, 0);
    if let Some(why) = mirror.diverged.clone() {
        // where, for the server's log
        let why = format!("turn {}, {:?}: {why}", game.turn, game.step);
        // the question Java is waiting on, if any, gets the news
        if !mirror.waiting {
            mirror.drain_to_ask();
        }
        let mut m = json!({"type": "diverged", "message": why});
        // a state that differs: Java's hash and this copy's text, so the server can log the lines that differ
        if let Some((java, rust)) = mirror.differs.take() {
            m["javaFp"] = json!(java);
            m["rustText"] = json!(rust);
        }
        mirror.send(m);
        return Err(why);
    }
    if !mirror.ended {
        mirror.send(json!({"type": "over"}));
        // Java's game ends a moment later: its last messages still find the process
        while let Some(Ok(line)) = mirror.lines.next() {
            match serde_json::from_str::<Message>(&line) {
                Ok(Message::End) => break,
                // Java's game goes on where this one ended: a difference
                Ok(Message::Ask { .. }) => {
                    mirror.send(json!({"type": "over"}));
                    break;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn send(out: &mut dyn Write, v: Value) {
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}

struct Mirror<'a> {
    lines: Box<dyn Iterator<Item = std::io::Result<String>> + 'a>,
    out: Box<dyn Write + 'a>,
    bot: Seat,
    table: Table,
    /// Java's fingerprints not yet compared, and this copy's
    java: VecDeque<String>,
    rust: VecDeque<String>,
    last: Option<String>,
    /// fingerprint texts, Java's (PLAYMAT_AI_DEBUG only) and this copy's
    texts: VecDeque<String>,
    rust_texts: VecDeque<String>,
    diverged: Option<String>,
    /// at a fingerprint that differs: Java's hash and this copy's text
    differs: Option<(String, String)>,
    ended: bool,
    asked: u64,
    /// Java is waiting on an answer this side couldn't give
    waiting: bool,
}

impl Mirror<'_> {
    fn send(&mut self, v: Value) {
        send(&mut self.out, v);
    }

    fn differ(&mut self, why: String) {
        if self.diverged.is_none() {
            self.diverged = Some(why);
        }
    }

    fn compare(&mut self) {
        while let (Some(j), Some(r)) = (self.java.front(), self.rust.front()) {
            if j != r {
                let why = format!("the state differs from Java's at a trace point (Java {j}, Rust {r})");
                if let (Some(jt), Some(rt)) = (self.texts.front(), self.rust_texts.front()) {
                    eprintln!("--- Java\n{jt}--- Rust\n{rt}");
                }
                if let Some(rt) = self.rust_texts.front() {
                    self.differs = Some((j.clone(), rt.clone()));
                }
                self.differ(why);
                return;
            }
            self.java.pop_front();
            self.rust.pop_front();
            self.texts.pop_front();
            self.rust_texts.pop_front();
        }
    }

    /// Reads up to Java's next question (after the copy stopped), so a `diverged` answers it.
    fn drain_to_ask(&mut self) {
        if self.ended {
            return;
        }
        for line in self.lines.by_ref() {
            let Ok(line) = line else { return };
            match serde_json::from_str::<Message>(&line) {
                Ok(Message::Ask { .. }) => return,
                Ok(Message::End) => {
                    self.ended = true;
                    return;
                }
                _ => {}
            }
        }
    }
}

impl Decider for Mirror<'_> {
    fn answer(&mut self, game: &Game, seat: Seat, q: &Query) -> Answer {
        loop {
            if self.diverged.is_some() || self.ended {
                return Answer::Stop;
            }
            let Some(Ok(line)) = self.lines.next() else {
                self.ended = true;
                return Answer::Stop;
            };
            if std::env::var_os("PLAYMAT_AI_DEBUG").is_some() {
                let pool = game.player(seat).mana_pool;
                eprintln!("seat {seat} (pool {pool:?}) asked {q:?}; Java says {line}");
            }
            let msg = match serde_json::from_str::<Message>(&line) {
                Ok(m) => m,
                Err(e) => {
                    self.differ(format!("bad message {line:?}: {e}"));
                    return Answer::Stop;
                }
            };
            match msg {
                Message::Start { .. } => self.differ("a second start".into()),
                Message::End => self.ended = true,
                Message::Trace { fp, text } => {
                    if let Some(t) = text {
                        self.texts.push_back(t);
                    }
                    self.java.push_back(fp);
                    self.compare();
                }
                Message::Decision { decision } => {
                    // which of several replacement effects applies first (Archon of Emeria's and a land's own "enters
                    // tapped", say): Java asks more often than the engine, which asks only where it owes the question;
                    // either order gives the same state, and a fingerprint shows it if not
                    let owed =
                        matches!(q, Query::Choice { message, .. } if message.starts_with("Choose a replacement"));
                    if !owed && decision.string.as_deref().is_some_and(replacement_key) {
                        continue;
                    }
                    if seat == self.bot || decision.seat != seat {
                        self.differ(format!(
                            "Java has seat {}'s answer, but here seat {seat} is asked {q:?}",
                            decision.seat
                        ));
                        continue;
                    }
                    let Some(a) = decode(game, &decision) else {
                        self.differ(format!("Java's answer {decision:?} names nothing here ({q:?})"));
                        continue;
                    };
                    if !q.accepts(&a) {
                        self.differ(format!("Java's answer {a:?} doesn't fit {q:?}"));
                        continue;
                    }
                    self.table.record(game, q, &a);
                    return a;
                }
                Message::Ask { question, follow } => {
                    self.waiting = true;
                    if seat != self.bot {
                        self.differ(format!("Java asks the AI ({question}), but here seat {seat} is asked {q:?}"));
                        continue;
                    }
                    let kind = question.split_whitespace().next().unwrap_or("");
                    // a choice the engine doesn't ask (a color or type for mana paying a generic cost): Java takes its
                    // first option; if that changes anything, the next fingerprint differs
                    if kind == "CHOOSE_CHOICE" && !matches!(q, Query::Choice { .. }) && seat == self.bot {
                        self.waiting = false;
                        // the color of "any color" mana the engine made for the one color owed
                        let color = match game.any_color_made {
                            Some(c) if question.contains("[White, Blue, Black, Red, Green]") => {
                                ["White", "Blue", "Black", "Red", "Green"][c.index()]
                            }
                            _ => "",
                        };
                        self.send(json!({"type": "answer", "string": color}));
                        continue;
                    }
                    if let Some(want) = java_kind(q) {
                        if !kind.is_empty() && !want.contains(&kind) {
                            self.differ(format!("Java asks the AI {question}, but here it's asked {q:?}"));
                            continue;
                        }
                    }
                    self.asked += 1;
                    let a = match follow {
                        Some(d) => match decode(game, &d) {
                            Some(a) => {
                                self.table.record(game, q, &a);
                                // what the AI would send for it must name the same thing
                                if let Some(v) = encode(game, seat, q, &a) {
                                    let back: Option<ShadowDecision> =
                                        serde_json::from_value(json!({"seat": seat, "target": v.get("target"),
                                            "bool": v.get("bool"), "integer": v.get("integer"),
                                            "string": v.get("string"), "manaType": v.get("manaType")}))
                                        .ok();
                                    if back.as_ref().and_then(|b| decode(game, b)).as_ref() != Some(&a) {
                                        self.differ(format!(
                                            "{a:?} goes to Java as {v} and doesn't come back the same"
                                        ));
                                        continue;
                                    }
                                }
                                let mut v = serde_json::to_value(&d).unwrap_or_default();
                                v["type"] = json!("answer");
                                self.waiting = false;
                                self.send(v);
                                return a;
                            }
                            None => {
                                self.differ(format!("the answer to follow {d:?} names nothing here"));
                                continue;
                            }
                        },
                        None => self.table.answer(game, seat, q),
                    };
                    match encode(game, seat, q, &a) {
                        Some(v) => {
                            self.waiting = false;
                            self.send(v)
                        }
                        None => {
                            self.differ(format!("no way to tell Java the answer {a:?} to {q:?}"));
                            continue;
                        }
                    }
                    return a;
                }
            }
        }
    }

    fn traced(&mut self, game: &Game) {
        if self.diverged.is_some() {
            return;
        }
        let text = fingerprint::text(game);
        let hash = fingerprint::hash_text(&text);
        if self.last.as_ref() == Some(&hash) {
            return;
        }
        self.rust_texts.push_back(text);
        self.last = Some(hash.clone());
        self.rust.push_back(hash);
        self.compare();
    }

    fn stopped(&self) -> bool {
        self.diverged.is_some() || self.ended
    }
}

/// The Java dialogs (`PlayerQueryEvent.QueryType`) a question can be; None: not checked
fn java_kind(q: &Query) -> Option<&'static [&'static str]> {
    Some(match q {
        Query::Priority { .. } | Query::Attackers { .. } | Query::Blockers { .. } => &["SELECT"],
        Query::PlayMana { .. } => &["PLAY_MANA"],
        Query::Target { .. } | Query::Cards { .. } => &["PICK_TARGET"],
        Query::YesNo { .. } => &["ASK"],
        Query::ChooseAbility { .. } => &["CHOOSE_ABILITY"],
        Query::ChooseMode { .. } => &["CHOOSE_MODE"],
        Query::Choice { .. } => &["CHOOSE_CHOICE"],
        Query::Amount { .. } => &["AMOUNT", "PLAY_X_MANA"],
        Query::MultiAmount { .. } => &["MULTI_AMOUNT"],
        _ => return None,
    })
}

/// A key of Java's replacement-effect choice: the effect's id and the source's, both UUIDs
fn replacement_key(s: &str) -> bool {
    let uuid = |p: &str| p.len() == 36 && p.chars().all(|c| c == '-' || c.is_ascii_hexdigit());
    s.split_once('_').is_some_and(|(a, b)| uuid(a) && uuid(b))
}

/// A Java answer as the engine's
pub fn decode(game: &Game, d: &ShadowDecision) -> Option<Answer> {
    Some(if let Some(r) = &d.target {
        resolve(game, r)?
    } else if let Some(s) = &d.string {
        Answer::Text(s.clone())
    } else if let Some(i) = d.integer {
        Answer::Int(i)
    } else if let Some(m) = &d.mana_type {
        Answer::ManaType(m.clone())
    } else {
        Answer::Bool(d.bool.unwrap_or(false))
    })
}

/// The engine's answer as a Java `DecisionLog` answer (`{"type":"answer", target|bool|integer|string|manaType}`)
pub fn encode(game: &Game, seat: Seat, _q: &Query, a: &Answer) -> Option<Value> {
    let v = match a {
        Answer::Bool(b) => json!({"bool": b}),
        Answer::Int(i) => json!({"integer": i}),
        Answer::Text(s) => json!({"string": s}),
        Answer::ManaType(m) => json!({"manaType": m}),
        Answer::Player(p) => json!({"target": format!("P{p}")}),
        Answer::Obj(o) => {
            let r = reference(game, *o);
            if r.starts_with('?') {
                return None;
            }
            json!({"target": r})
        }
        Answer::Ability(o, k) => {
            let r = reference(game, *o);
            if r.starts_with('?') {
                return None;
            }
            let room = !game.obj(*o).def.room.is_empty() && game.obj(*o).zone == Zone::Hand;
            json!({"target": format!("{r}{}{k}", if room { "h" } else { "a" })})
        }
        Answer::Trigger(i) => json!({"target": format!("T{seat}.{i}")}),
        Answer::Mode(k) => json!({"target": format!("S0sm{k}")}),
        Answer::ModeDone => json!({"target": "MD"}),
        Answer::ModeCancel => json!({"target": "MC"}),
        Answer::Unknown | Answer::Stop => return None,
    };
    let mut v = v;
    v["type"] = json!("answer");
    Some(v)
}
