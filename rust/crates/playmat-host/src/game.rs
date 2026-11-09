//! One game on the host: a thread runs the engine; the AI's seat is answered by the Rust AI, the human's by the
//! client's calls. What the client gets comes out as `Event`s, Java's callbacks (`GAME_INIT`, `GAME_UPDATE`, the
//! prompts, the game log's `CHATMESSAGE`s, `GAME_OVER`) with ids the client can send back.

use crate::ids::Ids;
use crate::prompts;
use crate::skips::{Passing, Settings};
use crate::timer::{Clock, Timer};
use playmat_ai::search::{Config, Table};
use playmat_engine::card::{CardDb, CardType};
use playmat_engine::decide::{Answer, Decider, Query};
use playmat_engine::rng::JavaRandom;
use playmat_engine::rules;
use playmat_engine::state::{Game, ObjId, Seat, Step, Zone};
use playmat_engine::view::game_view;
use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// A seat as Java's table gives it
#[derive(Clone, Debug)]
pub struct SeatStart {
    pub player_id: String,
    pub name: String,
    pub human: bool,
    /// the main deck's card names, one per card
    pub deck: Vec<String>,
    /// the player's `UserData` as JSON (skip settings, pass after casting…)
    pub user_data: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct Start {
    pub game_id: String,
    pub seed: i64,
    pub seats: Vec<SeatStart>,
    /// the table's priority timer
    pub timer: Timer,
}

/// What a client sends during the game (`GameApi`)
#[derive(Clone, Debug)]
pub enum Call {
    /// `gameJoin`: the client is ready (again, after a reconnect): the whole view and the open question
    Join,
    Uuid(Option<String>),
    Bool(bool),
    Int(i32),
    Str(String),
    ManaType(String),
    /// `sendPlayerAction`: the action and its data
    Action(String, Option<Value>),
    /// `gameResync`: the open question again
    Resync,
    /// `matchQuit`
    Quit,
}

/// A callback for the client: `{objectId, data, method}`; the gateway numbers it (`messageId`)
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub method: String,
    pub object_id: String,
    pub data: Value,
}

impl Event {
    pub fn json(&self) -> Value {
        json!({"objectId": self.object_id, "data": self.data, "method": self.method})
    }
}

/// How the game ended
#[derive(Clone, Debug)]
pub struct Ended {
    /// the winner's seat; none for a draw
    pub winner: Option<Seat>,
    pub turns: u32,
    /// why, for the server's log: "over", "conceded", "timed out", "error: …"
    pub reason: String,
}

pub struct HostedGame {
    pub game_id: String,
    pub chat_id: String,
    calls: Sender<Call>,
    pub events: Receiver<Event>,
    thread: Option<JoinHandle<Ended>>,
}

impl HostedGame {
    /// Deals the game and starts its thread; it waits for the human's `Join`.
    pub fn start(start: Start, db: Arc<CardDb>) -> Result<HostedGame, String> {
        Self::start_with(start, db, channel())
    }

    /// The same, with the channel its calls come in on (made before the game, so no call is lost while it starts)
    pub fn start_with(
        start: Start,
        db: Arc<CardDb>,
        calls: (Sender<Call>, Receiver<Call>),
    ) -> Result<HostedGame, String> {
        if start.seats.len() != 2 {
            return Err("the host plays duels only".into());
        }
        for s in &start.seats {
            if let Some(c) = s.deck.iter().find(|c| db.get(c).is_none()) {
                return Err(format!("card not in the engine: {c}"));
            }
        }
        let ids = Ids::new(&start.game_id, start.seats.iter().map(|s| s.player_id.clone()).collect());
        let chat_id = ids.chat();
        let (calls, calls_in) = calls;
        let (events_out, events) = channel();
        let game_id = start.game_id.clone();
        let chat = chat_id.clone();
        let thread = std::thread::Builder::new()
            .name(format!("host {}", start.game_id))
            .spawn(move || {
                let mut host = Host::new(start, &db, ids, chat, calls_in, events_out);
                let run = catch_unwind(AssertUnwindSafe(|| host.play()));
                match run {
                    Ok(ended) => ended,
                    Err(e) => {
                        let why = e
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                            .unwrap_or_default();
                        host.over(None, &format!("The game stopped on an error and ends as a draw. ({why})"));
                        Ended { winner: None, turns: host.game.turn, reason: format!("error: {why}") }
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(HostedGame { game_id, chat_id, calls, events, thread: Some(thread) })
    }

    /// Its parts, for a host that keeps the calls and the events apart: where calls go, the events, the thread
    pub fn split(mut self) -> (Sender<Call>, Receiver<Event>, JoinHandle<Ended>) {
        let thread = self.thread.take().expect("a started game");
        (self.calls, self.events, thread)
    }

    pub fn call(&self, c: Call) {
        let _ = self.calls.send(c);
    }

    /// Waits for the game's end (after its `GAME_OVER`)
    pub fn join(mut self) -> Ended {
        let thread = self.thread.take().expect("a started game");
        thread.join().unwrap_or(Ended { winner: None, turns: 0, reason: "panic".into() })
    }

    pub fn finished(&self) -> bool {
        self.thread.as_ref().is_none_or(|t| t.is_finished())
    }
}

/// How long a game waits for the human before it's given up (an abandoned game)
const IDLE: Duration = Duration::from_secs(30 * 60);

struct Host {
    game: Game,
    game_id: String,
    chat_id: String,
    names: Vec<String>,
    human: Seat,
    ai: Seat,
    settings: Settings,
    passing: Passing,
    ids: Ids,
    table: Table,
    calls: Receiver<Call>,
    events: Sender<Event>,
    joined: bool,
    /// the open question's callback, for a resync
    open: Option<Event>,
    log_seen: usize,
    conceded: Option<Seat>,
    /// the last view sent, so a state the client already has isn't sent again
    last_view: Option<Value>,
    /// auto-answers the player asked for (`REQUEST_AUTO_ANSWER_TEXT_*`): the question's text and the answer
    auto_answers: Vec<(String, bool)>,
    /// trigger orders the player set (`TRIGGER_AUTO_ORDER_*`): triggers by rule text (with their source's name)
    order_first: Vec<String>,
    order_last: Vec<String>,
    /// the rule texts of the triggers the open question asks to order, by place
    open_triggers: Vec<String>,
    /// the priority timer
    clock: Clock,
    /// the seat being asked, whose clock runs
    asking: Option<Seat>,
    /// the seat whose time ran out (it lost)
    timed_out: Option<Seat>,
}

impl Host {
    fn new(start: Start, db: &CardDb, ids: Ids, chat_id: String, calls: Receiver<Call>, events: Sender<Event>) -> Host {
        let human = start.seats.iter().position(|s| s.human).unwrap_or(0);
        let ai = 1 - human;
        let mut game = Game::new(2, JavaRandom::new(start.seed).state());
        for (seat, s) in start.seats.iter().enumerate() {
            let mut library = Vec::new();
            for name in &s.deck {
                let def = db.get(name).expect("checked");
                library.push(game.create_obj(def, seat, Zone::Library));
            }
            game.rng.shuffle(&mut library);
            game.player_mut(seat).library = library;
        }
        game.log = Some(Vec::new());
        let settings = Settings::parse(start.seats[human].user_data.as_ref());
        // the host decides when the human is asked
        game.player_mut(human).every_priority = true;
        game.player_mut(human).no_pool_check = !settings.confirm_empty_mana_pool;
        let mut ai_seats = vec![None, None];
        ai_seats[ai] = Some(Config::default());
        Host {
            game,
            game_id: start.game_id,
            chat_id,
            names: start.seats.iter().map(|s| s.name.clone()).collect(),
            human,
            ai,
            settings,
            passing: Passing::default(),
            ids,
            table: Table::new(ai_seats),
            calls,
            events,
            joined: false,
            open: None,
            log_seen: 0,
            conceded: None,
            last_view: None,
            auto_answers: Vec::new(),
            order_first: Vec::new(),
            order_last: Vec::new(),
            open_triggers: Vec::new(),
            clock: Clock::new(start.timer),
            asking: None,
            timed_out: None,
        }
    }

    fn send(&self, method: &str, object_id: &str, data: Value) {
        let _ = self.events.send(Event { method: method.into(), object_id: object_id.into(), data });
    }

    fn view(&mut self) -> Value {
        let names: Vec<&str> = self.names.iter().map(String::as_str).collect();
        let v = game_view(&self.game, self.human, &names);
        let mut v = self.ids.map(&self.game, v);
        if let Some(Value::Array(players)) = v.get_mut("players") {
            for (seat, p) in players.iter_mut().enumerate() {
                p["isHuman"] = json!(seat == self.human);
            }
        }
        v
    }

    /// The view with the timer's fields (`GameView.priorityTime`, `PlayerView.priorityTimeLeftSecs`…), which change
    /// with the time, so they're left out of the comparison with the last view sent
    fn timed(&self, mut v: Value) -> Value {
        let now = Instant::now();
        let timer = self.clock.timer();
        v["priorityTime"] = json!(timer.priority_secs);
        v["bufferTime"] = json!(timer.buffer_secs);
        let saved = now_ms();
        if let Some(Value::Array(players)) = v.get_mut("players") {
            for (seat, p) in players.iter_mut().enumerate() {
                let (left, buffer) = self.clock.view_secs(seat, now);
                let priority = p["hasPriority"].as_bool().unwrap_or(false);
                p["priorityTimeLeftSecs"] = json!(left);
                p["priorityTimeSavedTimeMs"] = json!(saved);
                p["bufferTimeLeft"] = json!(buffer);
                // Java: the player with priority, or the one choosing
                p["timerActive"] = json!(priority || self.asking == Some(seat));
            }
        }
        v
    }

    /// The open question again, its view's clocks as they are now
    fn send_open(&self) {
        if let Some(mut e) = self.open.clone() {
            if let Some(v) = e.data.get_mut("gameView").map(Value::take) {
                e.data["gameView"] = self.timed(v);
            }
            let _ = self.events.send(e);
        }
    }

    /// The game log's new lines, as the game chat's messages
    fn log(&mut self) {
        let lines: Vec<String> = match &self.game.log {
            Some(l) => l[self.log_seen.min(l.len())..].to_vec(),
            None => return,
        };
        self.log_seen += lines.len();
        let turn = format!("T{}.{}", self.game.turn, self.game.step.map(|s| s.name().to_string()).unwrap_or_default());
        for line in lines {
            let line = self.named(&line);
            let message = self.ids.text(&self.game, &line);
            self.send(
                "CHATMESSAGE",
                &self.chat_id.clone(),
                json!({"username": "", "message": message, "time": now_ms(), "turnInfo": turn,
                       "color": "BLACK", "messageType": "GAME"}),
            );
        }
    }

    /// Seat letters in the engine's lines (`A`, `B`) as the players' names
    fn named(&self, line: &str) -> String {
        let mut out = line.to_string();
        for (seat, name) in self.names.iter().enumerate() {
            let letter = (b'A' + seat as u8) as char;
            out = out.replace(
                &format!("<font color='#20B2AA'>{letter}</font>"),
                &format!("<font color='#20B2AA'>{name}</font>"),
            );
        }
        out
    }

    fn inform(&mut self, line: String) {
        if let Some(l) = self.game.log.as_mut() {
            l.push(line);
        }
        self.log();
    }

    /// `GAME_UPDATE` when the state the client has is out of date
    fn update(&mut self) {
        if !self.joined {
            return;
        }
        self.log();
        self.ids.clear();
        let view = self.view();
        if self.last_view.as_ref() != Some(&view) {
            self.last_view = Some(view.clone());
            self.send("GAME_UPDATE", &self.game_id.clone(), self.timed(view));
        }
    }

    /// Waits for the human to join (`gameJoin`), answering nothing else
    fn wait_join(&mut self) -> bool {
        while !self.joined {
            match self.calls.recv_timeout(IDLE) {
                Ok(Call::Join) => {
                    self.joined = true;
                    self.ids.clear();
                    let view = self.view();
                    self.last_view = Some(view.clone());
                    self.send("GAME_INIT", &self.game_id.clone(), self.timed(view));
                    self.log();
                }
                Ok(Call::Quit) | Ok(Call::Action(..)) => {
                    self.conceded = Some(self.human);
                    return false;
                }
                Ok(_) => {}
                Err(_) => {
                    self.conceded = Some(self.human);
                    return false;
                }
            }
        }
        true
    }

    /// Asks the human: renders the prompt with its view and waits for an answer that fits it. `None`: the human
    /// left (conceded, quit or gone).
    fn ask(&mut self, prompt: Value, accept: &dyn Fn(&Host, &Call) -> Option<Answer>) -> Option<Answer> {
        if !self.joined && !self.wait_join() {
            return None;
        }
        self.log();
        self.ids.clear();
        let view = self.view();
        let prompt = self.ids.map(&self.game, prompt);
        let method = prompt["method"].as_str().unwrap_or("GAME_SELECT").to_string();
        let mut data = prompt["data"].clone();
        // the human's clock runs while they're asked (`HumanPlayer.waitForResponse`)
        self.asking = Some(self.human);
        self.clock.resume(self.human, Instant::now());
        if let Some(d) = data.as_object_mut() {
            d.insert("gameView".into(), self.timed(view.clone()));
        }
        self.last_view = Some(view);
        if debug() {
            eprintln!(
                "host {} t{} {:?}: {} {}",
                self.game_id,
                self.game.turn,
                self.game.step,
                method,
                data["message"].as_str().unwrap_or("")
            );
        }
        let event = Event { method, object_id: self.game_id.clone(), data };
        self.open = Some(event.clone());
        let _ = self.events.send(event);
        let answer = self.wait(accept);
        self.clock.pause(Instant::now());
        self.asking = None;
        answer
    }

    /// The human's answer to the open question; `None` when they left, or their time ran out
    fn wait(&mut self, accept: &dyn Fn(&Host, &Call) -> Option<Answer>) -> Option<Answer> {
        loop {
            let wait = match self.clock.remaining(self.human, Instant::now()) {
                Some(left) => left.min(IDLE),
                None => IDLE,
            };
            let call = match self.calls.recv_timeout(wait) {
                Ok(c) => c,
                Err(RecvTimeoutError::Timeout) if self.clock.expired(self.human, Instant::now()) => {
                    self.time_out(self.human);
                    return None;
                }
                Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {
                    self.conceded = Some(self.human);
                    return None;
                }
            };
            if self.clock.expired(self.human, Instant::now()) {
                // an answer after the time ran out (Java's timer thread ends the game first)
                self.time_out(self.human);
                return None;
            }
            if debug() {
                // an id: what it names in the game
                let named = match &call {
                    Call::Uuid(Some(u)) => self.ids.ref_of(u).map(|r| {
                        let name = playmat_engine::replay::resolve(&self.game, r)
                            .and_then(|a| match a {
                                Answer::Obj(o) => Some(self.game.obj(o).name().to_string()),
                                _ => None,
                            })
                            .unwrap_or_default();
                        format!(" = {r} {name}")
                    }),
                    _ => None,
                };
                eprintln!("host {} <- {:?}{}", self.game_id, call, named.unwrap_or_default());
            }
            match &call {
                Call::Join => {
                    // a reconnect: the whole view, then the question again
                    let view = self.last_view.clone().unwrap_or(Value::Null);
                    self.send("GAME_INIT", &self.game_id.clone(), self.timed(view));
                    self.send_open();
                    continue;
                }
                Call::Resync => {
                    self.send_open();
                    continue;
                }
                Call::Quit => {
                    self.conceded = Some(self.human);
                    return None;
                }
                Call::Action(action, data) => match action.as_str() {
                    "CONCEDE" => {
                        self.conceded = Some(self.human);
                        return None;
                    }
                    a if a.starts_with("REQUEST_AUTO_ANSWER_") => {
                        self.auto_answer(a, data.as_ref());
                        continue;
                    }
                    a if a.starts_with("TRIGGER_AUTO_ORDER_") => {
                        self.trigger_order(a, data.as_ref());
                        continue;
                    }
                    a => {
                        let pass_now = self.passing.press(a, &self.game);
                        if pass_now {
                            if let Some(answer) = accept(self, &Call::Int(0)) {
                                self.open = None;
                                return Some(answer);
                            }
                        }
                        continue;
                    }
                },
                _ => {}
            }
            if let Some(answer) = accept(self, &call) {
                self.open = None;
                return Some(answer);
            }
            // an answer that doesn't fit: the question again (as Java re-asks)
            self.send_open();
        }
    }

    /// `PlayerImpl.timerTimeout`: the seat's time ran out, it loses
    fn time_out(&mut self, seat: Seat) {
        self.timed_out = Some(seat);
        self.conceded = Some(seat);
    }

    fn auto_answer(&mut self, action: &str, data: Option<&Value>) {
        match action {
            "REQUEST_AUTO_ANSWER_RESET_ALL" => self.auto_answers.clear(),
            "REQUEST_AUTO_ANSWER_TEXT_YES" | "REQUEST_AUTO_ANSWER_TEXT_NO" => {
                if let Some(text) = data.and_then(Value::as_str) {
                    self.auto_answers.retain(|(t, _)| t != text);
                    self.auto_answers.push((text.to_string(), action.ends_with("YES")));
                }
            }
            _ => {}
        }
    }

    /// `HumanPlayer.setTriggerAutoOrder`: a trigger of the open question (by id) or a rule text (by name) to put first
    /// or last from now on
    fn trigger_order(&mut self, action: &str, data: Option<&Value>) {
        if action == "TRIGGER_AUTO_ORDER_RESET_ALL" {
            self.order_first.clear();
            self.order_last.clear();
            return;
        }
        let Some(data) = data.and_then(Value::as_str) else { return };
        let rule = if action.starts_with("TRIGGER_AUTO_ORDER_ABILITY_") {
            match self.ids.answer(&self.game, data) {
                Some(Answer::Trigger(i)) => match self.open_triggers.get(i) {
                    Some(r) => r.clone(),
                    None => return,
                },
                _ => return,
            }
        } else {
            data.to_string()
        };
        self.order_first.retain(|r| *r != rule);
        self.order_last.retain(|r| *r != rule);
        if action.ends_with("_FIRST") {
            self.order_first.push(rule);
        } else {
            self.order_last.push(rule);
        }
    }

    fn auto_order(&self, rules: &[String]) -> Result<usize, Vec<usize>> {
        auto_order(&self.order_first, &self.order_last, rules)
    }

    /// The answer a call gives a question, if it fits
    fn answer_of(&self, q: &Query, call: &Call) -> Option<Answer> {
        let a = match call {
            Call::Uuid(Some(u)) => self.ids.answer(&self.game, u).unwrap_or(Answer::Unknown),
            // no id: a dialog's cancel or done
            Call::Uuid(None) => Answer::Bool(false),
            Call::Bool(b) => Answer::Bool(*b),
            Call::Int(i) => Answer::Int(*i),
            Call::Str(s) => Answer::Text(s.clone()),
            Call::ManaType(m) => Answer::ManaType(m.clone()),
            _ => return None,
        };
        q.accepts(&a).then_some(a)
    }

    fn play(&mut self) -> Ended {
        if !self.wait_join() {
            return self.finish();
        }
        let Some(first) = self.choose_starting_player() else { return self.finish() };
        for seat in [first, 1 - first] {
            rules::draw(&mut self.game, seat, 7);
        }
        if !self.mulligans(first) {
            return self.finish();
        }
        for seat in 0..2 {
            self.game.player_mut(seat).drawn = 0;
        }
        self.game.turn = 1;
        self.game.active = Some(first);
        self.game.priority = Some(first);
        self.game.step = Some(Step::Upkeep);
        self.update();
        let mut game = self.game.clone();
        {
            let mut d = HostDecider { host: self };
            rules::play_from_priority(&mut game, &mut d, 0);
        }
        self.game = game;
        self.finish()
    }

    /// The game's end: who won, `GAME_OVER`
    fn finish(&mut self) -> Ended {
        self.log();
        let (winner, reason) = if let Some(loser) = self.conceded {
            self.inform(format!("<font color='#20B2AA'>{}</font> has conceded.", self.names[loser]));
            if self.timed_out == Some(loser) {
                let line = format!(
                    "<font color='#20B2AA'>{}</font> has run out of time, losing the match.",
                    self.names[loser]
                );
                self.inform(line);
                (Some(1 - loser), "timed out".to_string())
            } else {
                (Some(1 - loser), "conceded".to_string())
            }
        } else if self.game.over {
            let alive: Vec<Seat> = (0..2).filter(|s| !self.game.player(*s).lost).collect();
            (if alive.len() == 1 { Some(alive[0]) } else { None }, "over".to_string())
        } else {
            (None, "stopped".to_string())
        };
        let message = match winner {
            Some(w) => format!("Player {} is the winner", self.names[w]),
            None => "Game is a draw".to_string(),
        };
        self.over(winner, &message);
        Ended { winner, turns: self.game.turn, reason }
    }

    fn over(&mut self, _winner: Option<Seat>, message: &str) {
        self.ids.clear();
        let view = self.view();
        let view = self.timed(view);
        self.send("GAME_OVER", &self.game_id.clone(), json!({"gameView": view, "message": message}));
    }

    /// `GameImpl.init`: a random player chooses who goes first; the AI always starts itself
    fn choose_starting_player(&mut self) -> Option<Seat> {
        let choosing = self.game.rng.next_int_bound(2) as Seat;
        if choosing == self.ai {
            let line =
                format!("<font color='#20B2AA'>{}</font> chooses that they take the first turn", self.names[self.ai]);
            self.inform(line);
            return Some(self.ai);
        }
        let prompt = prompts::starting_player(2);
        let a = self.ask(prompt, &|h, c| match h
            .answer_of(&Query::Target { players: vec![0, 1], objs: vec![], required: true }, c)
        {
            Some(Answer::Player(p)) => Some(Answer::Player(p)),
            _ => None,
        })?;
        let Answer::Player(first) = a else { return None };
        let line = if first == self.human {
            format!("<font color='#20B2AA'>{}</font> chooses that they take the first turn", self.names[first])
        } else {
            format!(
                "<font color='#20B2AA'>{}</font> chooses that <font color='#20B2AA'>{}</font> takes the first turn",
                self.names[self.human], self.names[first]
            )
        };
        self.inform(line);
        Some(first)
    }

    /// The London mulligan, starting player first; false when the human left
    fn mulligans(&mut self, first: Seat) -> bool {
        let mut kept = [false, false];
        let mut taken = [0usize, 0usize];
        while !(kept[0] && kept[1]) {
            let mut going = Vec::new();
            for seat in [first, 1 - first] {
                if kept[seat] {
                    continue;
                }
                let size = 7usize.saturating_sub(taken[seat] + 1);
                let wants = if taken[seat] >= 6 {
                    false
                } else if seat == self.human {
                    match self.ask(prompts::mulligan(size), &|_, c| match c {
                        Call::Bool(b) => Some(Answer::Bool(*b)),
                        _ => None,
                    }) {
                        Some(Answer::Bool(b)) => b,
                        _ => return false,
                    }
                } else {
                    ai_mulligans(&self.game, seat, taken[seat])
                };
                let name = self.names[seat].clone();
                if wants {
                    self.inform(format!("<font color='#20B2AA'>{name}</font> decides to take mulligan"));
                    going.push(seat);
                } else {
                    self.inform(format!("<font color='#20B2AA'>{name}</font> keeps hand"));
                    kept[seat] = true;
                }
            }
            for seat in going {
                taken[seat] += 1;
                let size = 7 - taken[seat];
                let hand = std::mem::take(&mut self.game.player_mut(seat).hand);
                for id in &hand {
                    self.game.obj_mut(*id).zone = Zone::Library;
                }
                let mut library = std::mem::take(&mut self.game.player_mut(seat).library);
                library.extend(hand);
                self.game.rng.shuffle(&mut library);
                self.game.player_mut(seat).library = library;
                rules::draw(&mut self.game, seat, 7);
                let name = self.names[seat].clone();
                self.inform(format!("<font color='#20B2AA'>{name}</font> mulligans down to {size} cards"));
                // the cards over the new size go to the bottom, one at a time
                while self.game.player(seat).hand.len() > size {
                    let more = self.game.player(seat).hand.len() - size;
                    let card = if seat == self.human {
                        let prompt = prompts::mulligan_bottom(&self.game, seat, more);
                        let hand = self.game.player(seat).hand.clone();
                        match self.ask(prompt, &|h, c| match h
                            .answer_of(&Query::Cards { objs: hand.clone(), required: true }, c)
                        {
                            Some(Answer::Obj(o)) if hand.contains(&o) => Some(Answer::Obj(o)),
                            _ => None,
                        }) {
                            Some(Answer::Obj(o)) => o,
                            _ => return false,
                        }
                    } else {
                        ai_bottom(&self.game, seat)
                    };
                    let p = self.game.player_mut(seat);
                    p.hand.retain(|x| *x != card);
                    p.library.push(card);
                    self.game.obj_mut(card).zone = Zone::Library;
                }
                self.update();
            }
        }
        true
    }
}

/// The engine's decider: the AI's seat to the Rust AI, the human's to the client
struct HostDecider<'a> {
    host: &'a mut Host,
}

impl Decider for HostDecider<'_> {
    fn answer(&mut self, game: &Game, seat: Seat, q: &Query) -> Answer {
        let h = &mut *self.host;
        if h.conceded.is_some() {
            return Answer::Stop;
        }
        if seat == h.ai {
            // the AI's clock runs while it thinks at priority (`ComputerPlayer7.priority`)
            if !matches!(q, Query::Priority { .. }) || !h.clock.on() {
                return h.table.answer(game, seat, q);
            }
            h.clock.resume(seat, Instant::now());
            let a = h.table.answer(game, seat, q);
            let now = Instant::now();
            h.clock.pause(now);
            if h.clock.expired(seat, now) {
                h.time_out(seat);
                return Answer::Stop;
            }
            return a;
        }
        // the host's game is the engine's game while it asks
        h.game = game.clone();
        if let Query::Priority { .. } = q {
            if h.passing.passes(game, seat, &h.settings) {
                h.table.record(game, q, &Answer::Bool(false));
                return Answer::Bool(false);
            }
        }
        if let Query::YesNo { message } = q {
            if let Some((_, yes)) = h.auto_answers.iter().find(|(t, _)| message.contains(t.as_str())) {
                let a = Answer::Bool(*yes);
                h.table.record(game, q, &a);
                return a;
            }
        }
        if let Query::PickTrigger { triggers, .. } = q {
            let rules: Vec<String> =
                triggers.iter().map(|(source, rule)| rule.replace("{this}", game.obj(*source).name())).collect();
            loop {
                let open = match h.auto_order(&rules) {
                    Ok(i) => {
                        let a = Answer::Trigger(i);
                        h.table.record(game, q, &a);
                        return a;
                    }
                    Err(open) => open,
                };
                // the question lists only the triggers with no order set
                let mut prompt = prompts::prompt(game, seat, q);
                for i in (0..rules.len()).filter(|i| !open.contains(i)) {
                    let r = format!("T{seat}.{i}");
                    if let Some(cards) = prompt["data"]["cardsView1"].as_object_mut() {
                        cards.remove(&r);
                    }
                    if let Some(t) = prompt["data"]["targets"].as_array_mut() {
                        t.retain(|x| x.as_str() != Some(r.as_str()));
                    }
                }
                h.open_triggers = rules.clone();
                let q2 = q.clone();
                // no id: "put it last" was just set, so the order is looked at again
                let answer = h.ask(prompt, &|h, c| match c {
                    Call::Uuid(None) => Some(Answer::Unknown),
                    _ => h.answer_of(&q2, c),
                });
                h.open_triggers.clear();
                match answer {
                    Some(Answer::Unknown) => continue,
                    Some(a) => {
                        h.table.record(game, q, &a);
                        return a;
                    }
                    None => return Answer::Stop,
                }
            }
        }
        let prompt = prompts::prompt(game, seat, q);
        let q2 = q.clone();
        let answer = h.ask(prompt, &|h, c| h.answer_of(&q2, c));
        match answer {
            Some(a) => {
                h.table.record(game, q, &a);
                a
            }
            None => Answer::Stop,
        }
    }

    fn traced(&mut self, game: &Game) {
        // the AI's plays and what resolves show as they happen
        let h = &mut *self.host;
        h.game = game.clone();
        h.update();
    }

    fn stopped(&self) -> bool {
        self.host.conceded.is_some()
    }
}

/// The AI's keep (the Java AI's rule of thumb): two to five lands in seven, fewer cards keep looser
fn ai_mulligans(game: &Game, seat: Seat, taken: usize) -> bool {
    let hand = &game.player(seat).hand;
    let lands = hand.iter().filter(|id| game.obj(**id).is(CardType::Land)).count();
    match taken {
        0 => !(2..=5).contains(&lands),
        1 => !(1..=5).contains(&lands),
        _ => false,
    }
}

/// The AI's card to the bottom: a land when it has more than three, else its most expensive spell
fn ai_bottom(game: &Game, seat: Seat) -> ObjId {
    let hand = &game.player(seat).hand;
    let lands: Vec<ObjId> = hand.iter().copied().filter(|id| game.obj(*id).is(CardType::Land)).collect();
    if lands.len() > 3 {
        return lands[0];
    }
    hand.iter()
        .copied()
        .filter(|id| !game.obj(*id).is(CardType::Land))
        .max_by_key(|id| game.obj(*id).def.mana_value())
        .unwrap_or(hand[0])
}

fn debug() -> bool {
    std::env::var_os("PLAYMAT_HOST_DEBUG").is_some()
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// `HumanPlayer.chooseTriggeredAbility`'s auto order: a trigger set to go first, else the only one not set to go last
/// (or the first of those set last); Err with the places the player has to choose among
fn auto_order(first: &[String], last: &[String], rules: &[String]) -> Result<usize, Vec<usize>> {
    if let Some(i) = rules.iter().position(|r| first.contains(r)) {
        return Ok(i);
    }
    let open: Vec<usize> = (0..rules.len()).filter(|i| !last.contains(&rules[*i])).collect();
    match open.len() {
        0 => Ok(0),
        1 => Ok(open[0]),
        _ => Err(open),
    }
}

#[cfg(test)]
mod tests {
    use super::auto_order;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn trigger_auto_order() {
        let rules = s(&["a", "b", "c"]);
        // nothing set: the player chooses among all
        assert_eq!(auto_order(&[], &[], &rules), Err(vec![0, 1, 2]));
        // one set first goes first
        assert_eq!(auto_order(&s(&["b"]), &[], &rules), Ok(1));
        // first wins over last
        assert_eq!(auto_order(&s(&["c"]), &s(&["c"]), &rules), Ok(2));
        // two set last: the one left goes
        assert_eq!(auto_order(&[], &s(&["a", "c"]), &rules), Ok(1));
        // one set last: the player chooses among the others
        assert_eq!(auto_order(&[], &s(&["a"]), &rules), Err(vec![1, 2]));
        // all set last: the first
        assert_eq!(auto_order(&[], &rules, &rules), Ok(0));
    }
}
