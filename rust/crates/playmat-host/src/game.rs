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
use playmat_engine::state::{EventKind, Game, ObjId, Seat, Step, Target, Zone};
use playmat_engine::view::game_view;
use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Where a game's log lines go as they're written, with their turn
pub type LogSink = Arc<std::sync::Mutex<Vec<(u32, String)>>>;

/// A seat as Java's table gives it
#[derive(Clone, Debug)]
pub struct SeatStart {
    pub player_id: String,
    pub name: String,
    pub human: bool,
    /// the main deck's card names, one per card
    pub deck: Vec<String>,
    /// each deck card's printing (`SET:number:imageNumber`), in the deck's order; empty: the engine's own
    pub printings: Vec<String>,
    /// the player's `UserData` as JSON (skip settings, pass after casting…)
    pub user_data: Option<Value>,
}

/// The table's options the game plays by (`MatchOptions`); Java keeps the games whose other options the host
/// doesn't play (`RustHost.host`)
#[derive(Clone, Debug, PartialEq)]
pub struct TableRules {
    /// each player's starting life (`Game.getStartingLife`)
    pub starting_life: i32,
    /// mulligans that don't cost a card (`Mulligan.getFreeMulligans`)
    pub free_mulligans: u32,
    /// the AI seat's skill, 1 to 10 (`TableController.skillOf`); none: the AI's full search
    pub ai_skill: Option<u32>,
    /// players may roll back to the start of a turn (`GameOptions.rollbackTurnsAllowed`)
    pub rollback: bool,
}

impl Default for TableRules {
    fn default() -> Self {
        TableRules { starting_life: 20, free_mulligans: 0, ai_skill: None, rollback: false }
    }
}

impl TableRules {
    /// The JSON `HOST_GAME` carries them as (`startingLife`, `freeMulligans`, `aiSkill`); a missing one is the default
    pub fn of(v: &Value) -> TableRules {
        let d = TableRules::default();
        TableRules {
            starting_life: v["startingLife"].as_i64().map(|n| n as i32).unwrap_or(d.starting_life),
            free_mulligans: v["freeMulligans"].as_u64().map(|n| n as u32).unwrap_or(d.free_mulligans),
            ai_skill: v["aiSkill"].as_u64().map(|n| n as u32),
            rollback: v["rollbackTurns"].as_bool().unwrap_or(d.rollback),
        }
    }

    pub fn json(&self) -> Value {
        json!({"startingLife": self.starting_life, "freeMulligans": self.free_mulligans, "aiSkill": self.ai_skill,
               "rollbackTurns": self.rollback})
    }

    /// The AI's search for its skill: Java's AI looks less far at a lower skill (`ComputerPlayer6.maxDepth`); the
    /// lowest skills play the quick answers only
    pub fn ai_config(&self) -> Option<Config> {
        let full = Config::default();
        match self.ai_skill {
            None => Some(full),
            Some(0..=1) => None,
            Some(2..=3) => Some(Config { budget: full.budget / 5, width: 5, ..full }),
            Some(4..=5) => Some(Config { budget: full.budget / 2, width: 8, ..full }),
            Some(_) => Some(full),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Start {
    pub game_id: String,
    pub seed: i64,
    pub seats: Vec<SeatStart>,
    /// starting life, free mulligans, the AI's skill
    pub rules: TableRules,
    /// the table's priority timer
    pub timer: Timer,
    /// the player id of who chooses who plays first: in a match, the previous game's loser; none for a random seat
    pub chooser: Option<String>,
    /// where the game's record goes (`record.rs`): its start, the players' calls, the log and the end
    pub record: Option<std::path::PathBuf>,
    /// where the game's replay goes (`replay.rs`), for the web client's History
    pub replay: Option<std::path::PathBuf>,
    /// the game log's lines as they're written, with their turn (a replay compares them with the record's)
    pub log_to: Option<LogSink>,
    /// a live game: answers the client sent before a question was asked are for an earlier one (a double click, an
    /// answer that crossed the AI's move) and are dropped. Off for a record's replay, whose calls are all queued at once.
    pub drop_stale: bool,
    /// a game that carries on from the start of a turn of an earlier one (a save, "Try this turn again", a what-if):
    /// the earlier game's start and calls, played without the client to that turn
    pub prelude: Option<Prelude>,
    /// the game's lines (`HOST_TURN`): at each turn's start, what a save of it needs (Java asks for them at solo
    /// tables)
    pub lines: bool,
}

impl Start {
    /// What object ids are made from: the game id of the line's first game, so a line's calls name the same objects
    /// in every game that carries it on
    pub fn ids_key(&self) -> &str {
        match &self.prelude {
            Some(p) => p.start.ids_key(),
            None => &self.game_id,
        }
    }
}

/// The start of a turn of a hosted game, as a later game carries it on: the line's first start, and every call since,
/// by seat, with the first start's player ids (`HOST_TURN`'s `line`)
#[derive(Clone, Debug)]
pub struct Prelude {
    pub start: Box<Start>,
    pub calls: Vec<(Seat, Call)>,
    /// the turn it carries on from: once the calls are played, the game goes live at that turn's first priority
    pub turn: u32,
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
    /// `gameAdvise`: what the AI would do in the player's place at the open question, at a skill (1 to 10), sent
    /// back as Java's `Advice`; nothing in the game changes
    Advise(u32, Sender<Value>),
    /// `gamePractice`: a practice tool (`HAND`, `BATTLEFIELD`, `DRAW`, `UNTAP_ALL`, `LIFE`) with its card and
    /// amount, run at the player's next priority; the reply says why not (empty: queued)
    Practice(String, Option<String>, i32, Option<Sender<String>>),
}

/// A callback for the client: `{objectId, data, method}`; the gateway numbers it (`messageId`)
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub method: String,
    pub object_id: String,
    pub data: Value,
    /// the seat it's for (its view, its question); none: every player's (the game log). `HOST_TURN` is the
    /// gateway's own.
    pub to: Option<Seat>,
}

impl Event {
    pub fn json(&self) -> Value {
        json!({"objectId": self.object_id, "data": self.data, "method": self.method})
    }
}

/// How the game ended
#[derive(Clone, Debug, Default)]
pub struct Ended {
    /// the winner's seat; none for a draw
    pub winner: Option<Seat>,
    pub turns: u32,
    /// why, for the server's log: "over", "conceded" (the game), "quit" (the match: `matchQuit`, or no answer for
    /// 30 minutes), "timed out" (the loser's priority timer ran out, which loses the match), "error: …". The one
    /// who conceded, quit or timed out is the loser.
    pub reason: String,
    /// each seat's priority time left in seconds, for the match's next game; none without a timer
    pub time_left: [Option<u32>; 2],
    /// what Java needs of the game's end besides the result: each seat's life and poison (`players`, by seat), the
    /// first human's Career tally (`tally`, `tally.rs`) and every human's (`tallies`, by player id); null when the
    /// game never started
    pub summary: Value,
}

/// Where a game's calls come in: each with the seat it's from
pub type Calls = (Sender<(Seat, Call)>, Receiver<(Seat, Call)>);

pub struct HostedGame {
    pub game_id: String,
    pub chat_id: String,
    calls: Sender<(Seat, Call)>,
    pub events: Receiver<Event>,
    thread: Option<JoinHandle<Ended>>,
    /// the first human's seat, whose calls `call` sends
    seat: Seat,
}

impl HostedGame {
    /// Deals the game and starts its thread; it waits for the humans' `Join`.
    pub fn start(start: Start, db: Arc<CardDb>) -> Result<HostedGame, String> {
        Self::start_with(start, db, channel())
    }

    /// The same, with the channel its calls come in on (made before the game, so no call is lost while it starts)
    pub fn start_with(start: Start, db: Arc<CardDb>, calls: Calls) -> Result<HostedGame, String> {
        if start.seats.len() != 2 {
            return Err("the host plays duels only".into());
        }
        if !start.seats.iter().any(|s| s.human) {
            return Err("nobody to play the game for".into());
        }
        for s in start.seats.iter().chain(start.prelude.iter().flat_map(|p| p.start.seats.iter())) {
            if let Some(c) = s.deck.iter().find(|c| db.get(c).is_none()) {
                return Err(format!("card not in the engine: {c}"));
            }
        }
        if let Some(p) = &start.prelude {
            let same = p.start.seats.len() == 2
                && p.start.seats.iter().zip(&start.seats).all(|(a, b)| a.human == b.human && a.deck == b.deck);
            if !same {
                return Err("the saved game's seats or decks aren't this table's".into());
            }
        }
        let seat = start.seats.iter().position(|s| s.human).unwrap_or(0);
        let chat_id = Ids::chat_of(&start.game_id);
        let (calls, calls_in) = calls;
        let (events_out, events) = channel();
        let game_id = start.game_id.clone();
        let chat = chat_id.clone();
        let thread = std::thread::Builder::new()
            .name(format!("host {}", start.game_id))
            .spawn(move || {
                let mut host = Host::new(start, &db, chat, calls_in, events_out);
                let run = catch_unwind(AssertUnwindSafe(|| host.play(&db)));
                let ended = match run {
                    Ok(ended) => ended,
                    Err(e) => {
                        let why = e
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                            .unwrap_or_default();
                        // as Java's `GameController.handleGameException`: the error, then the end
                        let error = format!(
                            "The game stopped on an error.\n\n{why}\nEngine: Rust host {}",
                            env!("CARGO_PKG_VERSION")
                        );
                        host.replaying = None;
                        for seat in host.humans() {
                            host.send(Some(seat), "GAME_ERROR", &host.game_id.clone(), Value::String(error.clone()));
                        }
                        host.over(None, &format!("The game stopped on an error and ends as a draw. ({why})"));
                        Ended {
                            winner: None,
                            turns: host.game.turn,
                            reason: format!("error: {why}"),
                            ..Ended::default()
                        }
                    }
                };
                if let Some(r) = host.record.as_mut() {
                    r.end(&ended);
                }
                ended
            })
            .map_err(|e| e.to_string())?;
        Ok(HostedGame { game_id, chat_id, calls, events, thread: Some(thread), seat })
    }

    /// Its parts, for a host that keeps the calls and the events apart: where calls go, the events, the thread
    pub fn split(mut self) -> (Sender<(Seat, Call)>, Receiver<Event>, JoinHandle<Ended>) {
        let thread = self.thread.take().expect("a started game");
        (self.calls, self.events, thread)
    }

    /// A call from the first human
    pub fn call(&self, c: Call) {
        let _ = self.calls.send((self.seat, c));
    }

    /// A call from a seat's player
    pub fn call_as(&self, seat: Seat, c: Call) {
        let _ = self.calls.send((seat, c));
    }

    /// Waits for the game's end (after its `GAME_OVER`)
    pub fn join(mut self) -> Ended {
        let thread = self.thread.take().expect("a started game");
        thread.join().unwrap_or(Ended { reason: "panic".into(), ..Ended::default() })
    }

    pub fn finished(&self) -> bool {
        self.thread.as_ref().is_none_or(|t| t.is_finished())
    }
}

/// How long a game waits for a player before it's given up (an abandoned game)
const IDLE: Duration = Duration::from_secs(30 * 60);

/// A seat's own side of the game: a human's settings, what they were sent, what they set during it
struct Player {
    human: bool,
    settings: Settings,
    passing: Passing,
    /// the ids of the player's last render
    ids: Ids,
    joined: bool,
    /// the open question's callback, for a resync
    open: Option<Event>,
    /// the last view sent, so a state the client already has isn't sent again
    last_view: Option<Value>,
    /// the last game event's number sent (`GameEventView.seq`)
    event_seq: u64,
    /// the events the client was sent (or that it had no use for) so far, as the engine recorded them
    events_sent: Vec<playmat_engine::state::GameEvent>,
    /// auto-answers the player asked for (`REQUEST_AUTO_ANSWER_TEXT_*`): the question's text and the answer
    auto_answers: Vec<(String, bool)>,
    /// trigger orders the player set (`TRIGGER_AUTO_ORDER_*`): triggers by rule text (with their source's name)
    order_first: Vec<String>,
    order_last: Vec<String>,
    /// the game before the mana the player made at priority since they last did anything else, for "Undo tap"
    /// (`PlayerImpl.storedBookmark`)
    undo_point: Option<Game>,
    /// what the player last did at priority (`PlayerImpl.justActivatedType`): the object they played and the stack's
    /// size then, for "pass priority after casting / activating"
    just_played: Option<(ObjId, usize)>,
    /// the seat's `UserData` as Java's `PlayerView.userData` shows it (the avatar); the AI's is the default
    user_data: Value,
}

/// A prelude being played: its calls left, and the turn the game goes live at
struct Replaying {
    calls: std::collections::VecDeque<(Seat, Call)>,
    turn: u32,
    /// the live game's start: its timer, settings and drop of late answers, taken on when it goes live
    live: Box<Start>,
}

struct Host {
    game: Game,
    record: Option<crate::record::Recorder>,
    replay: Option<crate::replay::ReplayWriter>,
    log_to: Option<LogSink>,
    drop_stale: bool,
    /// the calls other than answers that came in before a question, played at its start (`drop_stale`)
    held: std::collections::VecDeque<(Seat, Call)>,
    game_id: String,
    chat_id: String,
    names: Vec<String>,
    seats: Vec<Player>,
    /// the AI's seat; none in a game between people
    ai: Option<Seat>,
    /// the ids the game log's lines are written with
    log_ids: Ids,
    table: Table,
    calls: Receiver<(Seat, Call)>,
    events: Sender<Event>,
    log_seen: usize,
    conceded: Option<Seat>,
    /// the loser left the match, not only the game
    quit: bool,
    /// who chooses who plays first, when the table says
    chooser: Option<Seat>,
    /// the rule texts of the triggers the open question asks to order, by place
    open_triggers: Vec<String>,
    /// the open question, for advice
    open_query: Option<Query>,
    /// the priority timer
    clock: Clock,
    /// the seat being asked, whose clock runs
    asking: Option<Seat>,
    /// the seat whose time ran out (it lost)
    timed_out: Option<Seat>,
    /// each turn's game at its first priority, for a rollback (`GameImpl.gameStatesRollBack`, the last few)
    turn_starts: std::collections::BTreeMap<u32, Game>,
    rollback: bool,
    /// a rollback one player asked for and the other has yet to allow (`GameController.requestPermissionToRollback`):
    /// who asked, and to which turn
    rollback_request: Option<(Seat, u32)>,
    /// the game the engine plays on from instead (an undo, a rollback, a practice tool): the question being asked is
    /// dropped
    restart: Option<Game>,
    /// practice tools to run at the human's next priority: for whom, the tool, its card and amount
    practice: Vec<(Seat, String, Option<String>, i32)>,
    /// the asked player is being asked at priority
    at_priority: bool,
    free_mulligans: u32,
    /// the prelude being played (no client sees it); none once the game is live
    replaying: Option<Replaying>,
    /// the line so far, for `HOST_TURN`: its first start, and every call since by seat (the first start's ids)
    line_start: Option<Value>,
    line_calls: Vec<(Seat, Call)>,
    /// this game's player ids, and the line's first game's, to write calls in the line's ids
    players: Vec<String>,
    line_players: Vec<String>,
    /// the last turn whose start was seen
    turn_seen: u32,
}

impl Host {
    fn new(start: Start, db: &CardDb, chat_id: String, calls: Receiver<(Seat, Call)>, events: Sender<Event>) -> Host {
        // a prelude's game is the line's first one, played again: its seed, decks, rules and settings
        let first = start.prelude.as_ref().map(|p| (*p.start).clone()).unwrap_or_else(|| start.clone());
        let mut game = Game::new(2, JavaRandom::new(first.seed).state());
        game.starting_life = first.rules.starting_life;
        for seat in 0..2 {
            game.player_mut(seat).life = first.rules.starting_life;
        }
        for (seat, s) in first.seats.iter().enumerate() {
            let mut library = Vec::new();
            for (i, name) in s.deck.iter().enumerate() {
                let def = db.get(name).expect("checked");
                let id = game.create_obj(def, seat, Zone::Library);
                if let Some(p) = start.seats[seat].printings.get(i).filter(|p| !p.is_empty()) {
                    game.set_printing(id, p);
                }
                library.push(id);
            }
            game.rng.shuffle(&mut library);
            game.player_mut(seat).library = library;
        }
        game.log = Some(Vec::new());
        // the views carry the game's events, as `GameEventRecorder` feeds Java's (the client's effects play them)
        game.events = Some(Vec::new());
        let players: Vec<String> = start.seats.iter().map(|s| s.player_id.clone()).collect();
        let line_players: Vec<String> = first.seats.iter().map(|s| s.player_id.clone()).collect();
        let key = start.ids_key().to_string();
        let seats: Vec<Player> = first
            .seats
            .iter()
            .enumerate()
            .map(|(seat, s)| {
                let settings = Settings::parse(s.user_data.as_ref());
                Player {
                    human: s.human,
                    settings,
                    passing: Passing::default(),
                    ids: Ids::new(&key, players.clone()),
                    joined: false,
                    open: None,
                    last_view: None,
                    event_seq: 0,
                    events_sent: Vec::new(),
                    auto_answers: Vec::new(),
                    order_first: Vec::new(),
                    order_last: Vec::new(),
                    undo_point: None,
                    just_played: None,
                    user_data: start.seats[seat]
                        .user_data
                        .clone()
                        .filter(Value::is_object)
                        .unwrap_or_else(|| json!({"avatarId": 0})),
                }
            })
            .collect();
        for (seat, p) in seats.iter().enumerate() {
            if p.human {
                // the host decides when a human is asked
                game.player_mut(seat).every_priority = true;
                game.player_mut(seat).no_pool_check = !p.settings.confirm_empty_mana_pool;
                pool_payment(&game, seat, &p.settings);
            }
        }
        let ai = seats.iter().position(|p| !p.human);
        let mut ai_seats = vec![None, None];
        if let Some(ai) = ai {
            ai_seats[ai] = first.rules.ai_config();
        }
        let record = start.record.as_ref().and_then(|path| match crate::record::Recorder::create(path, &start) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("host {}: no record at {}: {e}", start.game_id, path.display());
                None
            }
        });
        let replay = start.replay.as_ref().and_then(|path| match crate::replay::ReplayWriter::create(path) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("host {}: no replay at {}: {e}", start.game_id, path.display());
                None
            }
        });
        let line_start = start.lines.then(|| crate::record::start_json(&first));
        let (replaying, line_calls) = match &start.prelude {
            Some(p) => {
                // the calls as this game's players: the line's player ids are this game's seats'
                let calls: std::collections::VecDeque<(Seat, Call)> =
                    p.calls.iter().map(|(s, c)| (*s, rename(c, &line_players, &players))).collect();
                let live = Box::new(Start { prelude: None, ..start.clone() });
                (Some(Replaying { calls, turn: p.turn, live }), p.calls.clone())
            }
            None => (None, Vec::new()),
        };
        Host {
            game,
            record,
            replay,
            log_to: start.log_to.clone(),
            drop_stale: start.drop_stale && replaying.is_none(),
            held: Default::default(),
            game_id: start.game_id.clone(),
            chat_id,
            names: start.seats.iter().map(|s| s.name.clone()).collect(),
            seats,
            ai,
            log_ids: Ids::new(&key, players.clone()),
            table: Table::new(ai_seats),
            calls,
            events,
            log_seen: 0,
            conceded: None,
            quit: false,
            chooser: first.chooser.as_ref().and_then(|c| first.seats.iter().position(|s| &s.player_id == c)),
            open_triggers: Vec::new(),
            open_query: None,
            // a prelude plays without the clock; the game's own starts when it goes live
            clock: Clock::new(if replaying.is_some() { Timer::default() } else { start.timer }),
            asking: None,
            timed_out: None,
            turn_starts: std::collections::BTreeMap::new(),
            rollback: first.rules.rollback,
            rollback_request: None,
            restart: None,
            practice: Vec::new(),
            at_priority: false,
            free_mulligans: first.rules.free_mulligans,
            replaying,
            line_start,
            line_calls,
            players,
            line_players,
            turn_seen: 0,
        }
    }

    /// The human seats
    fn humans(&self) -> Vec<Seat> {
        (0..self.seats.len()).filter(|s| self.seats[*s].human).collect()
    }

    /// The seat spectators and the replay see the board as: the first human's
    fn watch_seat(&self) -> Seat {
        self.seats.iter().position(|p| p.human).unwrap_or(0)
    }

    fn send(&self, to: Option<Seat>, method: &str, object_id: &str, data: Value) {
        if self.replaying.is_some() {
            return;
        }
        let _ = self.events.send(Event { method: method.into(), object_id: object_id.into(), data, to });
    }

    fn view(&mut self, seat: Seat) -> Value {
        let names: Vec<&str> = self.names.iter().map(String::as_str).collect();
        let v = game_view(&self.game, seat, &names);
        let mut v = self.seats[seat].ids.map(&self.game, v);
        // which engine plays the game (Java's views have none): the client shows that the Rust engine does
        v["engine"] = json!("rust");
        // a spell's card is the card it was in hand (the engine leaves it unnamed for the comparison with Java's,
        // whose spell has an id of its own): the client follows the card from hand to stack by it
        if let Some(Value::Object(stack)) = v.get_mut("stack") {
            for (id, card) in stack.iter_mut() {
                if card.get("cardId").and_then(Value::as_str) == Some("?") {
                    card["cardId"] = json!(id);
                }
            }
        }
        if let Some(Value::Array(players)) = v.get_mut("players") {
            for (s, p) in players.iter_mut().enumerate() {
                p["isHuman"] = json!(self.seats.get(s).is_some_and(|x| x.human));
                if let Some(x) = self.seats.get(s) {
                    p["userData"] = x.user_data.clone();
                }
            }
            // "Undo tap" (`PlayerView.statesSavedSize`): mana made at priority can be taken back
            if let Some(p) = players.get_mut(seat) {
                p["statesSavedSize"] = json!(usize::from(self.seats[seat].undo_point.is_some()));
            }
        }
        v["rollbackTurnsAllowed"] = json!(self.rollback);
        v
    }

    /// The game's events since the seat's last view, as `GameEventView`s for that player (`GameEventFeed.since`): a
    /// card that moved between zones they can't see (another player's draw) carries no identity. None: no new events
    fn take_events(&mut self, seat: Seat) -> Option<Value> {
        // the engine's game keeps every event (the host's game is a copy of it as it asks): what's new is what
        // follows the events sent already; an action taken back took its events back too
        let all = self.game.events.clone().unwrap_or_default();
        let p = &mut self.seats[seat];
        let same = p.events_sent.iter().zip(&all).take_while(|(a, b)| a == b).count();
        let mut events = all[same..].to_vec();
        p.events_sent = all;
        events.retain(|e| e.kind.shown());
        if events.is_empty() {
            return None;
        }
        let zone = |z: Option<Zone>| {
            z.map(|z| match z {
                Zone::Battlefield => "BATTLEFIELD",
                Zone::Graveyard => "GRAVEYARD",
                Zone::Stack => "STACK",
                Zone::Exile => "EXILED",
                Zone::Hand => "HAND",
                Zone::Library => "LIBRARY",
                Zone::Command => "COMMAND",
            })
        };
        let public = |z: Option<Zone>| !matches!(z, Some(Zone::Hand | Zone::Library) | None);
        let mut out = Vec::new();
        for e in events {
            let kind = match e.kind {
                EventKind::Damage => "DAMAGE",
                EventKind::LifeGain => "LIFE_GAIN",
                EventKind::LifeLoss => "LIFE_LOSS",
                EventKind::Draw => "DRAW",
                EventKind::Zone => "ZONE",
                _ => continue,
            };
            let p = &mut self.seats[seat];
            p.event_seq += 1;
            let seen = match e.kind {
                EventKind::Draw => e.player == Some(seat),
                EventKind::Zone => public(e.from) || public(e.to) || e.player == Some(seat),
                _ => true,
            };
            let mut v = json!({"seq": p.event_seq, "kind": kind, "amount": e.amount, "combat": e.combat});
            let ids = &p.ids;
            if let Some(id) = e.player.and_then(|s| ids.players.get(s)) {
                v["playerId"] = json!(id);
            }
            if let (Some(from), Some(to)) = (zone(e.from), zone(e.to)) {
                v["fromZone"] = json!(from);
                v["toZone"] = json!(to);
            }
            if seen {
                match e.target {
                    Some(Target::Player(s)) => {
                        v["targetId"] = json!(ids.players.get(s));
                        v["targetName"] = json!(self.names.get(s));
                    }
                    Some(Target::Obj(o)) => {
                        v["targetId"] = json!(ids.object(o));
                        v["targetName"] = json!(self.game.obj(o).name());
                    }
                    _ => {}
                }
                if let Some(s) = e.source {
                    v["sourceId"] = json!(ids.object(s));
                    v["sourceName"] = json!(self.game.obj(s).name());
                }
            }
            out.push(v);
        }
        Some(Value::Array(out))
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

    /// The seat's open question again, its view's clocks as they are now; with none, its board
    fn send_open(&self, seat: Seat) {
        match self.seats[seat].open.clone() {
            Some(mut e) => {
                if let Some(mut v) = e.data.get_mut("gameView").map(Value::take) {
                    // (its events were played already)
                    if let Some(m) = v.as_object_mut() {
                        m.remove("events");
                    }
                    e.data["gameView"] = self.timed(v);
                }
                if self.replaying.is_none() {
                    let _ = self.events.send(e);
                }
            }
            None => {
                if let Some(v) = self.seats[seat].last_view.clone() {
                    self.send(Some(seat), "GAME_UPDATE", &self.game_id, self.timed(v));
                }
            }
        }
    }

    /// The game log's new lines, as the game chat's messages
    fn log(&mut self) {
        let lines: Vec<String> = match &self.game.log {
            Some(l) => l[self.log_seen.min(l.len())..].to_vec(),
            None => return,
        };
        self.log_seen += lines.len();
        if self.replaying.is_some() {
            // a prelude's lines were the earlier game's
            return;
        }
        let turn = format!("T{}.{}", self.game.turn, self.game.step.map(|s| s.name().to_string()).unwrap_or_default());
        for line in lines {
            let line = self.named(&line);
            if let Some(r) = self.record.as_mut() {
                r.line(json!({"log": line, "turn": self.game.turn}));
            }
            if let Some(l) = &self.log_to {
                l.lock().unwrap().push((self.game.turn, line.clone()));
            }
            let message = self.log_ids.text(&self.game, &line);
            let data = json!({"username": "", "message": message, "time": now_ms(), "turnInfo": turn,
                   "color": "BLACK", "messageType": "GAME"});
            if let Some(r) = self.replay.as_mut() {
                r.log(&data);
            }
            self.send(None, "CHATMESSAGE", &self.chat_id.clone(), data);
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

    /// `GAME_UPDATE` to each player whose board is out of date
    fn update(&mut self) {
        self.update_except(None);
    }

    /// The same, but for the seat about to be asked (its question carries its board)
    fn update_except(&mut self, except: Option<Seat>) {
        if self.replaying.is_some() || !self.seats.iter().any(|p| p.joined) {
            return;
        }
        self.log();
        for seat in self.humans() {
            if !self.seats[seat].joined || Some(seat) == except {
                continue;
            }
            self.seats[seat].ids.clear();
            let view = self.view(seat);
            // (events with no change to show wait for the next view; an action taken back takes its events back too)
            if self.seats[seat].last_view.as_ref() != Some(&view) {
                let events = self.take_events(seat);
                self.seats[seat].last_view = Some(view.clone());
                let mut v = self.timed(view);
                if let Some(e) = events {
                    v["events"] = e;
                }
                self.send(Some(seat), "GAME_UPDATE", &self.game_id.clone(), v);
            }
        }
        if except.is_none() {
            self.replay_frame();
        }
    }

    /// The board as it is now, in the replay: the first human's view, every human's hand beside it
    fn replay_frame(&mut self) {
        if self.replaying.is_some() || self.replay.is_none() {
            return;
        }
        let watch = self.watch_seat();
        let Some(view) = self.seats[watch].last_view.clone() else { return };
        let hands: Vec<(String, Value)> = self
            .humans()
            .into_iter()
            .map(|s| {
                let hand = self.seats[s].last_view.as_ref().map(|v| v["myHand"].clone()).unwrap_or(json!({}));
                (self.names[s].clone(), hand)
            })
            .collect();
        if let Some(r) = self.replay.as_mut() {
            r.frame(&view, &hands);
        }
    }

    /// A player joins (or joins again): the whole board, and their open question
    fn join(&mut self, seat: Seat) {
        if !self.seats[seat].human {
            return;
        }
        if self.seats[seat].joined {
            // a reconnect: the whole view, then the question again
            let view = self.seats[seat].last_view.clone().unwrap_or(Value::Null);
            self.send(Some(seat), "GAME_INIT", &self.game_id.clone(), self.timed(view));
            if self.seats[seat].open.is_some() {
                self.send_open(seat);
            }
            return;
        }
        self.seats[seat].joined = true;
        self.seats[seat].ids.clear();
        let view = self.view(seat);
        // its events so far were before the player came
        self.seats[seat].events_sent = self.game.events.clone().unwrap_or_default();
        self.seats[seat].last_view = Some(view.clone());
        self.send(Some(seat), "GAME_INIT", &self.game_id.clone(), self.timed(view));
        self.log();
    }

    /// The next call: one held for the question, a prelude's, or one from a client
    fn next_call(&mut self, wait: Duration) -> Result<(Seat, Call), RecvTimeoutError> {
        if let Some(c) = self.held.pop_front() {
            return Ok(c);
        }
        if let Some(r) = self.replaying.as_mut() {
            // the prelude's calls only: a client's wait for the game to go live
            return r.calls.pop_front().ok_or(RecvTimeoutError::Disconnected);
        }
        self.calls.recv_timeout(wait)
    }

    /// A call taken: into the record, and into the game's line
    fn took(&mut self, from: Seat, call: &Call) {
        if self.replaying.is_some() {
            return;
        }
        if matches!(call, Call::Join | Call::Resync | Call::Advise(..)) {
            if let Some(r) = self.record.as_mut() {
                r.call(from, call, self.game.turn, None);
            }
            return;
        }
        if let Some(r) = self.record.as_mut() {
            let reference = match call {
                Call::Uuid(Some(u)) => self.seats[from].ids.ref_of(u),
                _ => None,
            };
            r.call(from, call, self.game.turn, reference);
        }
        if self.line_start.is_some() {
            self.line_calls.push((from, rename(call, &self.players, &self.line_players)));
        }
    }

    /// Waits for every human to join (`gameJoin`); false when one left first
    fn wait_joined(&mut self) -> bool {
        while let Some(missing) = self.humans().into_iter().find(|s| !self.seats[*s].joined) {
            let (from, call) = match self.next_call(IDLE) {
                Ok(c) => c,
                Err(_) => {
                    self.conceded = Some(missing);
                    self.quit = true;
                    return false;
                }
            };
            self.took(from, &call);
            if let Call::Join = call {
                self.join(from);
            } else if self.side_call(from, call) {
                return false;
            }
        }
        true
    }

    /// Asks a seat's player: renders the prompt with its view and waits for an answer that fits it. `None`: the game
    /// stops (a player conceded, quit or is gone; an undo or a rollback).
    fn ask(&mut self, seat: Seat, prompt: Value, accept: &dyn Fn(&Host, &Call) -> Option<Answer>) -> Option<Answer> {
        if self.replaying.is_none() && self.humans().iter().any(|s| !self.seats[*s].joined) && !self.wait_joined() {
            return None;
        }
        self.update_except(Some(seat));
        self.log();
        self.seats[seat].ids.clear();
        let view = self.view(seat);
        let prompt = self.seats[seat].ids.map(&self.game, prompt);
        let method = prompt["method"].as_str().unwrap_or("GAME_SELECT").to_string();
        let mut data = prompt["data"].clone();
        // the player's clock runs while they're asked (`HumanPlayer.waitForResponse`)
        self.asking = Some(seat);
        self.clock.resume(seat, Instant::now());
        let events = if self.replaying.is_none() { self.take_events(seat) } else { None };
        if let Some(d) = data.as_object_mut() {
            let mut v = self.timed(view.clone());
            if let Some(e) = events {
                v["events"] = e;
            }
            d.insert("gameView".into(), v);
        }
        self.seats[seat].last_view = Some(view);
        self.replay_frame();
        if debug() && self.replaying.is_none() {
            eprintln!(
                "host {} t{} {:?} seat {seat}: {} {}",
                self.game_id,
                self.game.turn,
                self.game.step,
                method,
                data["message"].as_str().unwrap_or("")
            );
        }
        let event = Event { method, object_id: self.game_id.clone(), data, to: Some(seat) };
        if self.drop_stale {
            self.drop_stale_answers();
        }
        self.seats[seat].open = Some(event.clone());
        if self.replaying.is_none() {
            let _ = self.events.send(event);
        }
        let answer = self.wait(seat, accept);
        self.clock.pause(Instant::now());
        self.asking = None;
        answer
    }

    /// The answers already sent, before the question about to be asked: for an earlier one, so dropped (Java's
    /// `HumanPlayer` drops a response nobody waits for). The other calls wait for the question.
    fn drop_stale_answers(&mut self) {
        while let Ok((from, call)) = self.calls.try_recv() {
            match call {
                Call::Uuid(_) | Call::Bool(_) | Call::Int(_) | Call::Str(_) | Call::ManaType(_) => {
                    if debug() {
                        eprintln!("host {} dropped a late answer {:?} from seat {from}", self.game_id, call);
                    }
                }
                other => self.held.push_back((from, other)),
            }
        }
    }

    /// A call from a player who isn't being asked: their settings, a concede, a reconnect, a rollback's consent. True
    /// when the game stops for it.
    fn side_call(&mut self, from: Seat, call: Call) -> bool {
        if !self.seats[from].human {
            return false;
        }
        match call {
            Call::Join => self.join(from),
            Call::Resync => self.send_open(from),
            Call::Quit => {
                self.conceded = Some(from);
                self.quit = true;
                return true;
            }
            Call::Advise(_, reply) => {
                let _ = reply.send(advice_none("Ask while the game waits for you."));
            }
            Call::Practice(tool, card, amount, reply) => {
                let problem = self.queue_practice(from, tool, card, amount);
                if let Some(r) = reply {
                    let _ = r.send(problem);
                }
            }
            Call::Action(action, data) => match action.as_str() {
                "CONCEDE" => {
                    self.conceded = Some(from);
                    return true;
                }
                "ADD_PERMISSION_TO_ROLLBACK_TURN" => {
                    if let Some((by, turns)) = self.rollback_request.filter(|(by, _)| *by != from) {
                        self.rollback_request = None;
                        let _ = by;
                        return self.roll_back(from, turns);
                    }
                }
                "DENY_PERMISSION_TO_ROLLBACK_TURN" => {
                    if self.rollback_request.is_some_and(|(by, _)| by != from) {
                        self.rollback_request = None;
                        let name = self.names[from].clone();
                        self.inform(format!("Rollback request denied by <font color='#20B2AA'>{name}</font>"));
                    }
                }
                "ROLLBACK_TURNS" => {
                    self.inform_seat(from, "You can only request a rollback if you have priority.");
                }
                "UNDO" => {}
                a if a.starts_with("REQUEST_AUTO_ANSWER_") => self.auto_answer(from, a, data.as_ref()),
                a if a.starts_with("TRIGGER_AUTO_ORDER_") && !a.starts_with("TRIGGER_AUTO_ORDER_ABILITY_") => {
                    self.trigger_order(from, a, data.as_ref())
                }
                a if self.seats[from].settings.press(a) => pool_payment(&self.game, from, &self.seats[from].settings),
                a => {
                    let game = &self.game;
                    self.seats[from].passing.press(a, game);
                }
            },
            // an answer from a player nobody asked: dropped (Java's `HumanPlayer` drops it)
            _ => {}
        }
        false
    }

    /// The asked player's answer to the open question; `None` when the game stops (someone left, their time ran out,
    /// an undo, a rollback)
    fn wait(&mut self, seat: Seat, accept: &dyn Fn(&Host, &Call) -> Option<Answer>) -> Option<Answer> {
        loop {
            let wait = match self.clock.remaining(seat, Instant::now()) {
                Some(left) => left.min(IDLE),
                None => IDLE,
            };
            let (from, call) = match self.next_call(wait) {
                Ok(c) => c,
                Err(RecvTimeoutError::Timeout) if self.clock.expired(seat, Instant::now()) => {
                    self.time_out(seat);
                    return None;
                }
                Err(_) if self.replaying.is_some() => {
                    // the prelude's calls ran out before its turn: the game goes live here, at this question
                    self.go_live();
                    if !self.wait_joined() {
                        return None;
                    }
                    self.send_open(seat);
                    continue;
                }
                Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {
                    self.conceded = Some(seat);
                    self.quit = true;
                    return None;
                }
            };
            self.took(from, &call);
            if self.clock.expired(seat, Instant::now()) {
                // an answer after the time ran out (Java's timer thread ends the game first)
                self.time_out(seat);
                return None;
            }
            if debug() && self.replaying.is_none() {
                // an id: what it names in the game
                let named = match &call {
                    Call::Uuid(Some(u)) => self.seats[from].ids.ref_of(u).map(|r| {
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
                eprintln!("host {} <- seat {from} {:?}{}", self.game_id, call, named.unwrap_or_default());
            }
            if from != seat {
                if self.side_call(from, call) {
                    return None;
                }
                continue;
            }
            match &call {
                Call::Join => {
                    self.join(seat);
                    continue;
                }
                Call::Resync => {
                    self.send_open(seat);
                    continue;
                }
                Call::Quit => {
                    self.conceded = Some(seat);
                    self.quit = true;
                    return None;
                }
                Call::Advise(skill, reply) => {
                    let advice = self.advise(seat, *skill);
                    let _ = reply.send(advice);
                    continue;
                }
                Call::Practice(tool, card, amount, reply) => {
                    let problem = self.queue_practice(seat, tool.clone(), card.clone(), *amount);
                    if let Some(r) = reply {
                        let _ = r.send(problem.clone());
                    }
                    // at priority it runs now: the game plays on from it
                    if problem.is_empty() && self.at_priority {
                        self.restart = Some(self.game.clone());
                        return None;
                    }
                    continue;
                }
                Call::Action(action, data) => match action.as_str() {
                    "CONCEDE" => {
                        self.conceded = Some(seat);
                        return None;
                    }
                    "UNDO" => {
                        // `GameImpl.undo`: the mana made since the last action is taken back
                        if self.at_priority {
                            if let Some(g) = self.seats[seat].undo_point.take() {
                                self.restart = Some(g);
                                return None;
                            }
                        }
                        continue;
                    }
                    "ROLLBACK_TURNS" => {
                        if !self.at_priority {
                            self.inform_seat(seat, "You can only request a rollback if you have priority.");
                            continue;
                        }
                        let back = data.as_ref().and_then(Value::as_u64).unwrap_or(0) as u32;
                        let others: Vec<Seat> = self.humans().into_iter().filter(|s| *s != seat).collect();
                        if others.is_empty() {
                            // against the AI: nobody else to ask
                            if self.roll_back(seat, back) {
                                return None;
                            }
                        } else if self.rollback_state(back).is_none() {
                            self.inform_seat(seat, "That turn is not available for rollback.");
                        } else {
                            // `GameController.requestPermissionToRollback`: the other player allows it, or not
                            self.rollback_request = Some((seat, back));
                            for other in others {
                                self.ask_rollback(seat, other, back);
                            }
                        }
                        continue;
                    }
                    a if a.starts_with("REQUEST_AUTO_ANSWER_") => {
                        self.auto_answer(seat, a, data.as_ref());
                        continue;
                    }
                    a if a.starts_with("TRIGGER_AUTO_ORDER_") => {
                        self.trigger_order(seat, a, data.as_ref());
                        continue;
                    }
                    a if self.seats[seat].settings.press(a) => {
                        // `GameImpl.setManaPaymentMode` (the engine's game shares it with this copy)
                        pool_payment(&self.game, seat, &self.seats[seat].settings);
                        continue;
                    }
                    a => {
                        let game = &self.game;
                        let pass_now = self.seats[seat].passing.press(a, game);
                        if pass_now {
                            if let Some(answer) = accept(self, &Call::Int(0)) {
                                self.seats[seat].open = None;
                                return Some(answer);
                            }
                        }
                        continue;
                    }
                },
                _ => {}
            }
            if let Some(answer) = accept(self, &call) {
                self.seats[seat].open = None;
                return Some(answer);
            }
            // an answer that doesn't fit: the question again (as Java re-asks)
            self.send_open(seat);
        }
    }

    /// `GameSessionPlayer.requestPermissionToRollbackTurn`: the other player's dialog
    fn ask_rollback(&mut self, by: Seat, to: Seat, back: u32) {
        let message = match back {
            0 => "Allow rollback to the start of the current turn?".to_string(),
            1 => "Allow rollback to the start of the previous turn?".to_string(),
            n => format!("Allow to rollback {n} turns?"),
        };
        let data = json!({"title": format!("Request by {}", self.names[by]), "message": message,
            "relatedUserId": self.players.get(by), "relatedUserName": self.names[by], "gameId": self.game_id,
            "button1Text": "Accept", "button1Action": "ADD_PERMISSION_TO_ROLLBACK_TURN",
            "button2Text": "Deny", "button2Action": "DENY_PERMISSION_TO_ROLLBACK_TURN"});
        self.send(Some(to), "USER_REQUEST_DIALOG", &self.game_id.clone(), data);
    }

    /// The game at the start of the turn so many turns ago (0: this one), when it can be gone back to
    fn rollback_state(&self, back: u32) -> Option<Game> {
        let turn = self.game.turn.saturating_sub(back);
        if self.rollback && turn > 0 {
            self.turn_starts.get(&turn).cloned()
        } else {
            None
        }
    }

    /// `GameController`'s `ROLLBACK_TURNS`, asked for by `by` (and allowed by the others): back to the start of the
    /// turn the data says, so many turns ago (0: this one); true when the game goes back
    fn roll_back(&mut self, by: Seat, back: u32) -> bool {
        let turn = self.game.turn.saturating_sub(back);
        let Some(state) = self.rollback_state(back) else {
            self.inform_seat(by, "That turn is not available for rollback.");
            return false;
        };
        // `GameImpl.rollbackTurnsExecution`
        self.inform(format!("<font color='#FF8C00'>Player request: Rolling back to start of turn {turn}</font>"));
        self.turn_starts.split_off(&(turn + 1));
        for p in &mut self.seats {
            p.undo_point = None;
        }
        self.restart = Some(state);
        true
    }

    /// A line in one player's game log only (`Game.informPlayer`)
    fn inform_seat(&mut self, seat: Seat, line: &str) {
        let turn = format!("T{}.{}", self.game.turn, self.game.step.map(|s| s.name().to_string()).unwrap_or_default());
        let data = json!({"username": "", "message": line, "time": now_ms(), "turnInfo": turn,
               "color": "BLACK", "messageType": "GAME"});
        self.send(Some(seat), "CHATMESSAGE", &self.chat_id.clone(), data);
    }

    /// `PlayerImpl.timerTimeout`: the seat's time ran out, it loses
    fn time_out(&mut self, seat: Seat) {
        self.timed_out = Some(seat);
        self.conceded = Some(seat);
    }

    fn auto_answer(&mut self, seat: Seat, action: &str, data: Option<&Value>) {
        let answers = &mut self.seats[seat].auto_answers;
        match action {
            "REQUEST_AUTO_ANSWER_RESET_ALL" => answers.clear(),
            "REQUEST_AUTO_ANSWER_TEXT_YES" | "REQUEST_AUTO_ANSWER_TEXT_NO" => {
                if let Some(text) = data.and_then(Value::as_str) {
                    answers.retain(|(t, _)| t != text);
                    answers.push((text.to_string(), action.ends_with("YES")));
                }
            }
            _ => {}
        }
    }

    /// `HumanPlayer.setTriggerAutoOrder`: a trigger of the open question (by id) or a rule text (by name) to put first
    /// or last from now on
    fn trigger_order(&mut self, seat: Seat, action: &str, data: Option<&Value>) {
        if action == "TRIGGER_AUTO_ORDER_RESET_ALL" {
            self.seats[seat].order_first.clear();
            self.seats[seat].order_last.clear();
            return;
        }
        let Some(data) = data.and_then(Value::as_str) else { return };
        let rule = if action.starts_with("TRIGGER_AUTO_ORDER_ABILITY_") {
            match self.seats[seat].ids.answer(&self.game, data) {
                Some(Answer::Trigger(i)) => match self.open_triggers.get(i) {
                    Some(r) => r.clone(),
                    None => return,
                },
                _ => return,
            }
        } else {
            data.to_string()
        };
        let p = &mut self.seats[seat];
        p.order_first.retain(|r| *r != rule);
        p.order_last.retain(|r| *r != rule);
        if action.ends_with("_FIRST") {
            p.order_first.push(rule);
        } else {
            p.order_last.push(rule);
        }
    }

    /// The answer a call gives the open question, if it fits (the ids are the asked player's)
    fn answer_of(&self, q: &Query, call: &Call) -> Option<Answer> {
        let ids = &self.seats[self.asking.unwrap_or_else(|| self.watch_seat())].ids;
        let a = match call {
            Call::Uuid(Some(u)) => ids.answer(&self.game, u).unwrap_or(Answer::Unknown),
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

    /// The prelude is played: the game is live from here, with this game's timer, settings and drop of late answers;
    /// its players join it
    fn go_live(&mut self) {
        let Some(r) = self.replaying.take() else { return };
        let live = r.live;
        self.clock = Clock::new(live.timer);
        self.drop_stale = live.drop_stale;
        for (seat, s) in live.seats.iter().enumerate() {
            let p = &mut self.seats[seat];
            if !p.human {
                continue;
            }
            // the player's settings now (the prelude played by the earlier game's)
            p.settings = Settings::parse(s.user_data.as_ref());
            p.passing = Passing::default();
            self.game.player_mut(seat).no_pool_check = !p.settings.confirm_empty_mana_pool;
            pool_payment(&self.game, seat, &p.settings);
        }
        self.log_seen = self.game.log.as_ref().map_or(0, Vec::len);
        let turn = self.game.turn;
        self.inform(format!("The game carries on from the start of turn {turn}."));
    }

    /// `HOST_TURN`: the line to this turn's start, for Java's saves (a solo table's)
    fn turn_line(&mut self, active: Option<Seat>) {
        let Some(start) = self.line_start.clone() else { return };
        if self.replaying.is_some() {
            return;
        }
        let calls: Vec<Value> = self
            .line_calls
            .iter()
            .map(|(s, c)| json!({"seat": s, "call": crate::record::call_json(c)}))
            .collect();
        let line = json!({"start": start, "calls": calls, "turn": self.game.turn});
        let data = json!({"turn": self.game.turn, "activePlayerId": active.and_then(|s| self.players.get(s)),
            "line": line});
        self.send(None, "HOST_TURN", &self.game_id.clone(), data);
    }

    /// A practice tool for the seat's player, queued for their next priority; why not, or empty
    fn queue_practice(&mut self, seat: Seat, tool: String, card: Option<String>, amount: i32) -> String {
        if self.ai.is_none() || !self.seats[seat].human {
            return "Practice tools only work in a game against the computer.".into();
        }
        let tool = tool.trim().to_uppercase();
        match tool.as_str() {
            "HAND" | "BATTLEFIELD" | "DRAW" if !(1..=10).contains(&amount) => {
                return "Add or draw 1 to 10 cards at a time.".into();
            }
            "HAND" | "BATTLEFIELD" | "DRAW" | "UNTAP_ALL" => {}
            "LIFE" if !(1..=999).contains(&amount) => return "Life must be 1 to 999.".into(),
            "LIFE" => {}
            _ => return format!("Unknown practice tool: {tool}"),
        }
        self.practice.push((seat, tool, card, amount));
        String::new()
    }

    /// `gameAdvise`: what the AI would do in the seat's place at the open question (`AdviceService.advise`)
    fn advise(&mut self, seat: Seat, skill: u32) -> Value {
        if self.ai.is_none() {
            return advice_none("Advice only works in a game against the computer.");
        }
        let Some(q) = self.open_query.clone() else {
            return advice_none("Ask while the game waits for you to act.");
        };
        let rules = TableRules { ai_skill: Some(skill), ..TableRules::default() };
        let mut seats = vec![None, None];
        seats[seat] = rules.ai_config();
        let mut advisor = Table::new(seats);
        let began = Instant::now();
        let game = self.game.clone();
        let a = advisor.answer(&game, seat, &q);
        let millis = began.elapsed().as_millis() as u64;
        let ids = &self.seats[seat].ids;
        let step = |text: String, o: ObjId| json!({"text": text, "objectId": ids.object(o), "targetIds": []});
        let name = |o: ObjId| game.obj(o).name().to_string();
        let (kind, text, steps) = match (&q, &a) {
            (Query::Priority { .. }, Answer::Obj(o)) => {
                let verb = if game.obj(*o).is(CardType::Land) && game.obj(*o).zone == Zone::Hand {
                    "Play"
                } else if game.obj(*o).zone == Zone::Battlefield {
                    "Use"
                } else {
                    "Cast"
                };
                let t = format!("{verb} {}", name(*o));
                ("PLAY", t.clone(), vec![step(t, *o)])
            }
            (Query::Priority { .. }, Answer::Ability(o, _)) => {
                let t = format!("Activate {}", name(*o));
                ("PLAY", t.clone(), vec![step(t, *o)])
            }
            (Query::Priority { .. }, _) => ("PASS", "Pass: nothing worth doing now.".to_string(), vec![]),
            (Query::Attackers { .. }, Answer::Obj(o)) => {
                let t = format!("{} attacks", name(*o));
                ("ATTACK", format!("Attack with {}", name(*o)), vec![step(t, *o)])
            }
            (Query::Attackers { .. }, _) => ("NO_ATTACK", "Don't attack.".to_string(), vec![]),
            (Query::Blockers { .. }, Answer::Obj(o)) => {
                let t = format!("{} blocks", name(*o));
                ("BLOCK", format!("Block with {}", name(*o)), vec![step(t, *o)])
            }
            (Query::Blockers { .. }, _) => ("NO_BLOCK", "Don't block.".to_string(), vec![]),
            (_, Answer::Obj(o)) => {
                let t = format!("Choose {}", name(*o));
                ("PLAY", t.clone(), vec![step(t, *o)])
            }
            (_, Answer::Player(p)) => ("PLAY", format!("Choose {}", self.names[*p]), vec![]),
            (_, Answer::Bool(b)) => ("PLAY", if *b { "Answer yes." } else { "Answer no." }.to_string(), vec![]),
            _ => ("NONE", "The AI has nothing to say about this question.".to_string(), vec![]),
        };
        let step_name = game.step.map(|s| s.name().to_string()).unwrap_or_default();
        json!({"kind": kind, "text": text, "steps": steps, "millis": millis,
            "when": format!("Turn {}, {}", game.turn, step_name.to_lowercase()),
            "reason": "The Rust AI's pick in your place."})
    }

    fn play(&mut self, db: &CardDb) -> Ended {
        if self.replaying.is_none() && !self.wait_joined() {
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
            // the opening hands' actions (Devourer of Destiny's reveal, Gemstone Caverns), then the first turn
            if rules::opening_hand_actions(&mut game, &mut d, first) {
                game.priority = Some(first);
                rules::play_from_priority(&mut game, &mut d, 0);
            }
        }
        // an undo, a rollback or a practice tool: the engine plays on from the game it goes back to, from its
        // priority question
        while let Some(back) = self.restart.take() {
            if self.conceded.is_some() {
                break;
            }
            game = back;
            let practice = std::mem::take(&mut self.practice);
            if !practice.is_empty() {
                let mut d = HostDecider { host: self };
                run_practice(&mut game, practice, db, &mut d);
            }
            self.game = game.clone();
            self.update();
            let mut d = HostDecider { host: self };
            rules::play_from_priority_question(&mut game, &mut d, 0);
        }
        self.game = game;
        self.finish()
    }

    /// The game's end: who won, `GAME_OVER`
    fn finish(&mut self) -> Ended {
        // a game that ended in its prelude (the saved game was over): its players see the end
        if self.replaying.is_some() {
            self.go_live();
        }
        let summary = self.summary();
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
                (Some(1 - loser), if self.quit { "quit" } else { "conceded" }.to_string())
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
        let now = Instant::now();
        self.clock.pause(now);
        let time_left = [self.clock.left_secs(0, now), self.clock.left_secs(1, now)];
        let mut summary = summary;
        // the replay is complete: Java lists it
        summary["replayEvents"] = json!(self.replay.as_mut().map(|r| r.finish()).unwrap_or(0));
        Ended { winner, turns: self.game.turn, reason, time_left, summary }
    }

    /// The end's `summary` (`Ended`), each human's tally counted from the game's whole history
    fn summary(&mut self) -> Value {
        let mut tallies = serde_json::Map::new();
        let mut first = Value::Null;
        for seat in self.humans() {
            let mut tally = crate::tally::Tally::new(seat, self.game.starting_life);
            tally.watch(&self.game, self.game.events.as_deref().unwrap_or_default());
            let report = tally.report(&self.game);
            if first.is_null() {
                first = report.clone();
            }
            tallies.insert(self.players[seat].clone(), report);
        }
        let players: Vec<Value> = (0..2)
            .map(|s| {
                let p = self.game.player(s);
                let poison: i32 = p.counters.iter().filter(|(n, _)| n == "poison").map(|(_, k)| *k).sum();
                json!({"life": p.life, "poison": poison})
            })
            .collect();
        json!({"players": players, "tally": first, "tallies": tallies})
    }

    fn over(&mut self, _winner: Option<Seat>, message: &str) {
        for seat in self.humans() {
            self.seats[seat].ids.clear();
            let view = self.view(seat);
            self.seats[seat].last_view = Some(view.clone());
            let view = self.timed(view);
            self.send(Some(seat), "GAME_OVER", &self.game_id.clone(), json!({"gameView": view, "message": message}));
        }
        self.replay_frame();
    }

    /// `GameImpl.init`: the table's chooser (a match's last loser), else a random player, chooses who goes first; the
    /// AI always starts itself
    fn choose_starting_player(&mut self) -> Option<Seat> {
        let choosing = match self.chooser {
            Some(seat) => seat,
            None => self.game.rng.next_int_bound(2) as Seat,
        };
        if !self.seats[choosing].human {
            let line =
                format!("<font color='#20B2AA'>{}</font> chooses that they take the first turn", self.names[choosing]);
            self.inform(line);
            return Some(choosing);
        }
        let prompt = prompts::starting_player(2);
        let a = self.ask(choosing, prompt, &|h, c| match h
            .answer_of(&Query::Target { players: vec![0, 1], objs: vec![], required: true }, c)
        {
            Some(Answer::Player(p)) => Some(Answer::Player(p)),
            _ => None,
        })?;
        let Answer::Player(first) = a else { return None };
        let line = if first == choosing {
            format!("<font color='#20B2AA'>{}</font> chooses that they take the first turn", self.names[first])
        } else {
            format!(
                "<font color='#20B2AA'>{}</font> chooses that <font color='#20B2AA'>{}</font> takes the first turn",
                self.names[choosing], self.names[first]
            )
        };
        self.inform(line);
        Some(first)
    }

    /// The London mulligan, starting player first; false when a player left
    fn mulligans(&mut self, first: Seat) -> bool {
        let mut kept = [false, false];
        let mut taken = [0usize, 0usize];
        // `LondonMulligan`: the first free ones keep the hand's size
        let free = self.free_mulligans as usize;
        let size_after = |taken: usize| 7usize.saturating_sub(taken.saturating_sub(free));
        while !(kept[0] && kept[1]) {
            let mut going = Vec::new();
            for seat in [first, 1 - first] {
                if kept[seat] {
                    continue;
                }
                let size = size_after(taken[seat] + 1);
                let wants = if size_after(taken[seat]) == 0 {
                    false
                } else if self.seats[seat].human {
                    match self.ask(seat, prompts::mulligan(size), &|_, c| match c {
                        Call::Bool(b) => Some(Answer::Bool(*b)),
                        _ => None,
                    }) {
                        Some(Answer::Bool(b)) => b,
                        _ => return false,
                    }
                } else {
                    ai_mulligans(&self.game, seat, taken[seat].saturating_sub(free))
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
                let size = size_after(taken[seat]);
                let for_free = taken[seat] <= free;
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
                if for_free {
                    self.inform(format!("<font color='#20B2AA'>{name}</font> mulligans for free."));
                } else {
                    let cards = if size == 1 { "card" } else { "cards" };
                    self.inform(format!("<font color='#20B2AA'>{name}</font> mulligans down to {size} {cards}"));
                }
                // the cards over the new size go to the bottom, one at a time
                while self.game.player(seat).hand.len() > size {
                    let more = self.game.player(seat).hand.len() - size;
                    let card = if self.seats[seat].human {
                        let prompt = prompts::mulligan_bottom(&self.game, seat, more);
                        let hand = self.game.player(seat).hand.clone();
                        match self.ask(seat, prompt, &|h, c| match h
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

/// A call with one game's player ids for another's (a line's calls, played in a later game)
fn rename(call: &Call, from: &[String], to: &[String]) -> Call {
    match call {
        Call::Uuid(Some(u)) => match from.iter().position(|p| p == u) {
            Some(seat) => Call::Uuid(to.get(seat).cloned().or(Some(u.clone()))),
            None => call.clone(),
        },
        c => c.clone(),
    }
}

/// Runs practice tools on the game (`PracticeTools.action`), each announced in the log
fn run_practice(game: &mut Game, tools: Vec<(Seat, String, Option<String>, i32)>, db: &CardDb, d: &mut dyn Decider) {
    for (seat, tool, card, amount) in tools {
        let who = playmat_engine::log::player(seat);
        let n = amount.max(0) as usize;
        let line = match tool.as_str() {
            "HAND" | "BATTLEFIELD" => match card.as_deref().and_then(|c| db.get(c.trim())) {
                None => format!("Practice: the Rust engine has no card named {}.", card.unwrap_or_default()),
                Some(def) => {
                    let name = def.name.clone();
                    for _ in 0..n {
                        if tool == "HAND" {
                            let id = game.create_obj(def.clone(), seat, Zone::Hand);
                            game.player_mut(seat).hand.push(id);
                        } else {
                            let id = game.create_obj(def.clone(), seat, Zone::Exile);
                            rules::put_onto_battlefield(game, d, id, seat);
                        }
                    }
                    let to = if tool == "HAND" { "into their hand" } else { "onto the battlefield" };
                    format!("Practice: {who} puts {amount} {name} {to}.")
                }
            },
            "DRAW" => {
                rules::draw(game, seat, n);
                format!("Practice: {who} draws {amount} {}.", if amount == 1 { "card" } else { "cards" })
            }
            "UNTAP_ALL" => {
                let mine: Vec<ObjId> =
                    game.battlefield.iter().copied().filter(|id| game.obj(*id).controller == seat).collect();
                for id in mine {
                    game.obj_mut(id).tapped = false;
                }
                format!("Practice: {who} untaps everything they control.")
            }
            "LIFE" => {
                game.player_mut(seat).life = amount;
                format!("Practice: {who} sets their life to {amount}.")
            }
            _ => continue,
        };
        if let Some(l) = game.log.as_mut() {
            l.push(line);
        }
    }
}

/// Java's `Advice.none`
fn advice_none(why: &str) -> Value {
    json!({"kind": "NONE", "text": why, "steps": []})
}

/// The engine's decider: the AI's seat to the Rust AI, the humans' to their clients
struct HostDecider<'a> {
    host: &'a mut Host,
}

impl Decider for HostDecider<'_> {
    fn answer(&mut self, game: &Game, seat: Seat, q: &Query) -> Answer {
        let h = &mut *self.host;
        if h.conceded.is_some() || h.restart.is_some() {
            return Answer::Stop;
        }
        if let Query::Priority { .. } = q {
            // a prelude played to its turn: the game goes live at that turn's first priority
            let at_turn = h.replaying.as_ref().is_some_and(|r| r.calls.is_empty() && game.turn >= r.turn);
            if at_turn {
                h.game = game.clone();
                h.go_live();
                if !h.wait_joined() {
                    return Answer::Stop;
                }
            }
            // each turn's first priority, for a rollback (`GameImpl.saveRollBackGameState`, the last four turns)
            if h.rollback && !h.turn_starts.contains_key(&game.turn) {
                h.turn_starts.insert(game.turn, game.clone());
                while h.turn_starts.len() > 5 {
                    h.turn_starts.pop_first();
                }
            }
            // a new turn's start (a turn rolled back to starts again): the line to it, for a save
            if game.turn != h.turn_seen {
                h.turn_seen = game.turn;
                h.game = game.clone();
                h.turn_line(game.active);
            }
        }
        // a turn another player controls (Emrakul, the Promised End): the controller has the player do nothing it
        // needn't; what it must answer, the quick policy answers
        if game.controlled_by.iter().any(|(p, c)| *p == seat && *c != seat) {
            if let Query::Priority { .. } = q {
                return Answer::Bool(false);
            }
            return h.table.answer(game, seat, q);
        }
        if !h.seats[seat].human {
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
            // a practice tool waits for the player's priority: the game plays on from it
            if !h.practice.is_empty() && h.replaying.is_none() {
                h.restart = Some(game.clone());
                return Answer::Stop;
            }
            // `HumanPlayer.priority`: a spell just cast (an ability just activated) passes at once when the player's
            // settings say so, unless they held priority for it
            if let Some((obj, before)) = h.seats[seat].just_played.take() {
                let item = game.stack.iter().skip(before).find(|i| i.card == obj && i.controller == seat);
                let settings = &h.seats[seat].settings;
                let pass = match item {
                    Some(i) if i.ability.is_none() && i.delayed.is_none() => settings.pass_after_cast,
                    Some(_) => settings.pass_after_activation,
                    None => false,
                };
                if pass && !h.seats[seat].passing.holding {
                    h.seats[seat].passing.holding = false;
                    h.table.record(game, q, &Answer::Bool(false));
                    return Answer::Bool(false);
                }
            }
            // held for one action: the next priority asks again
            h.seats[seat].passing.holding = false;
            let p = &mut h.seats[seat];
            if p.passing.passes(game, seat, &p.settings) {
                h.table.record(game, q, &Answer::Bool(false));
                return Answer::Bool(false);
            }
        }
        // `useFirstManaAbility`: a permanent with several mana abilities uses its first
        if let Query::ChooseAbility { source, abilities } = q {
            let def = &game.obj(*source).def;
            if h.seats[seat].settings.use_first_mana_ability
                && abilities.len() > 1
                && abilities.iter().all(|i| def.abilities.get(*i).is_some_and(is_mana_ability))
            {
                let a = Answer::Ability(*source, abilities[0]);
                h.table.record(game, q, &a);
                return a;
            }
        }
        if let Query::YesNo { message } = q {
            if let Some((_, yes)) = h.seats[seat].auto_answers.iter().find(|(t, _)| message.contains(t.as_str())) {
                let a = Answer::Bool(*yes);
                h.table.record(game, q, &a);
                return a;
            }
        }
        if let Query::PickTrigger { triggers, .. } = q {
            let rules: Vec<String> =
                triggers.iter().map(|(source, rule)| rule.replace("{this}", game.obj(*source).name())).collect();
            loop {
                let open = match auto_order(&h.seats[seat].order_first, &h.seats[seat].order_last, &rules) {
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
                let answer = h.ask(seat, prompt, &|h, c| match c {
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
        h.at_priority = matches!(q, Query::Priority { .. });
        h.open_query = Some(q.clone());
        let answer = h.ask(seat, prompt, &|h, c| h.answer_of(&q2, c));
        h.open_query = None;
        h.at_priority = false;
        match answer {
            Some(a) => {
                h.table.record(game, q, &a);
                if let Query::Priority { playable } = q {
                    if let Answer::Obj(o) | Answer::Ability(o, _) = &a {
                        h.seats[seat].just_played = Some((*o, game.stack.len()));
                    }
                    // a permanent clicked for its mana: undoable until the player does anything else
                    let mana = matches!(a, Answer::Obj(o) if !playable.contains(&o) && game.battlefield.contains(&o));
                    let p = &mut h.seats[seat];
                    if !mana {
                        p.undo_point = None;
                    } else if p.undo_point.is_none() {
                        p.undo_point = Some(game.clone());
                    }
                }
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
        self.host.conceded.is_some() || self.host.restart.is_some()
    }
}

/// Whether an ability is a mana ability (its variant's name: `…ManaAbility`)
fn is_mana_ability(a: &playmat_engine::card::AbilityDef) -> bool {
    let name = format!("{a:?}");
    name.split(|c: char| !c.is_alphanumeric()).next().is_some_and(|v| v.ends_with("ManaAbility"))
}

/// How the player's pool pays (`ManaPool.autoPayment`, `autoPaymentRestricted`): never by itself, by itself once mana
/// was made for the payment (the engine's stock), or always
fn pool_payment(game: &Game, seat: Seat, settings: &Settings) {
    game.set_pool_mode(seat, !settings.pool_automatic, settings.pool_automatic && !settings.pool_restricted);
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
