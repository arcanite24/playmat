//! Whole games on the host: a scripted client (plays lands, never casts, keeps its hand) against the Rust AI,
//! through the client's protocol only. Every game must end with `GAME_OVER`, and every id the client gets must be
//! one it can send back.

use playmat_ai::duel::Deck;
use playmat_host::{Call, Event, HostedGame, SeatStart, Start, Timer};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

fn deck(file: &str) -> Deck {
    let path = format!("{}/../playmat-engine/shadow-decks/tier30/{file}", env!("CARGO_MANIFEST_DIR"));
    Deck::parse(file, &std::fs::read_to_string(path).unwrap())
}

fn uuid(s: &str) -> bool {
    s.len() == 36 && s.chars().filter(|c| *c == '-').count() == 4
}

/// Every id-shaped text in a prompt: its targets, choices, attackers and blockers
fn ids_of(e: &Event) -> Vec<String> {
    let d = &e.data;
    let mut out: Vec<String> = Vec::new();
    if let Some(a) = d["targets"].as_array() {
        out.extend(a.iter().filter_map(|v| v.as_str().map(str::to_string)));
    }
    for key in ["possibleAttackers", "possibleBlockers", "possibleTargets"] {
        if let Some(a) = d["options"][key].as_array() {
            out.extend(a.iter().filter_map(|v| v.as_str().map(str::to_string)));
        }
    }
    if let Some(m) = d["choices"].as_object() {
        out.extend(m.keys().cloned());
    }
    out
}

struct Bot {
    tried: HashSet<String>,
    turn: u64,
    concede_at: u64,
    /// never answers at priority (the game's priority timer runs out)
    silent: bool,
    /// casts what it can from hand in its main phases, as well as playing lands
    casts: bool,
    /// holds priority (HOLD_PRIORITY) for each card it plays
    holds: bool,
}

impl Bot {
    fn answer(&mut self, e: &Event) -> Option<Call> {
        let d = &e.data;
        let view = &d["gameView"];
        // (an update's view is its data: only a question's view says the turn)
        let turn = view["turn"].as_u64().unwrap_or(self.turn);
        if turn != self.turn {
            self.turn = turn;
            self.tried.clear();
        }
        Some(match e.method.as_str() {
            "GAME_ASK" => {
                let pool = d["message"].as_str().unwrap_or("").contains("mana pool");
                Call::Bool(pool)
            }
            "GAME_TARGET" => {
                let t: Vec<&str> = d["targets"].as_array()?.iter().filter_map(Value::as_str).collect();
                Call::Uuid(t.first().map(|s| s.to_string()))
            }
            "GAME_SELECT" if self.silent => return None,
            "GAME_SELECT" => {
                let options = &d["options"];
                if options.get("possibleAttackers").is_some() || options.get("possibleBlockers").is_some() {
                    return Some(Call::Bool(true));
                }
                if turn >= self.concede_at {
                    return Some(Call::Action("CONCEDE".into(), None));
                }
                // a land from hand, once per card a turn
                let me = view["myPlayerId"].as_str().unwrap_or("");
                let mine = view["activePlayerId"].as_str() == Some(me);
                let main = view["step"].as_str().is_some_and(|s| s.contains("MAIN"));
                let empty = view["stack"].as_object().is_none_or(|m| m.is_empty());
                if mine && main && empty {
                    if let Some(hand) = view["myHand"].as_object() {
                        for (id, card) in hand {
                            let land = card["cardTypes"].as_array().is_some_and(|t| t.iter().any(|x| x == "LAND"));
                            if land && self.tried.insert(id.clone()) {
                                return Some(Call::Uuid(Some(id.clone())));
                            }
                        }
                        let playable = view["canPlayObjects"]["objects"].as_object();
                        for id in hand.keys() {
                            if self.casts
                                && playable.is_some_and(|p| p.contains_key(id))
                                && self.tried.insert(id.clone())
                            {
                                return Some(Call::Uuid(Some(id.clone())));
                            }
                        }
                    }
                }
                Call::Bool(false)
            }
            "GAME_CHOOSE_ABILITY" => Call::Uuid(d["choices"].as_object()?.keys().next().cloned()),
            "GAME_CHOOSE_CHOICE" => Call::Str(d["choice"]["choices"][0].as_str().unwrap_or("").to_string()),
            "GAME_GET_AMOUNT" => Call::Int(d["min"].as_i64().unwrap_or(0) as i32),
            "GAME_GET_MULTI_AMOUNT" => Call::Str(
                d["messages"]
                    .as_array()?
                    .iter()
                    .map(|m| m["defaultValue"].as_i64().unwrap_or(0).to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            // an untapped land of its own pays, when it casts; else the payment is cancelled
            "GAME_PLAY_MANA" if self.casts => {
                let me = view["myPlayerId"].as_str().unwrap_or("");
                let lands = view["players"].as_array()?.iter().find(|p| p["playerId"] == me)?["battlefield"]
                    .as_object()?
                    .iter()
                    .filter(|(_, c)| {
                        c["tapped"] != true && c["cardTypes"].as_array().is_some_and(|t| t.iter().any(|x| x == "LAND"))
                    })
                    .map(|(id, _)| id.clone())
                    .next();
                match lands {
                    Some(id) => Call::Uuid(Some(id)),
                    None => Call::Bool(false),
                }
            }
            "GAME_PLAY_MANA" | "GAME_PLAY_XMANA" => Call::Bool(false),
            _ => return None,
        })
    }
}

const HUMAN: &str = "11111111-1111-4111-8111-111111111111";
const AI: &str = "22222222-2222-4222-8222-222222222222";

fn play(human_deck: &Deck, ai_deck: &Deck, seed: i64, concede_at: u64) -> (playmat_host::Ended, Vec<Event>) {
    play_timed(human_deck, ai_deck, seed, concede_at, Timer::default(), false)
}

fn play_timed(
    human_deck: &Deck,
    ai_deck: &Deck,
    seed: i64,
    concede_at: u64,
    timer: Timer,
    silent: bool,
) -> (playmat_host::Ended, Vec<Event>) {
    play_start(Start { timer, ..start(human_deck, ai_deck, seed, None) }, concede_at, silent)
}

fn play_start(start: Start, concede_at: u64, silent: bool) -> (playmat_host::Ended, Vec<Event>) {
    play_with(start, Bot { tried: HashSet::new(), turn: 0, concede_at, silent, casts: false, holds: false })
}

fn play_with(start: Start, mut bot: Bot) -> (playmat_host::Ended, Vec<Event>) {
    let game = HostedGame::start(start, Arc::new(playmat_engine::cards())).unwrap();
    game.call(Call::Join);
    let mut events = Vec::new();
    loop {
        let e = game.events.recv_timeout(Duration::from_secs(60)).expect("the host answers within a minute");
        events.push(e.clone());
        if e.method == "GAME_OVER" {
            break;
        }
        for id in ids_of(&e) {
            assert!(uuid(&id), "{} offers {id}, not an id the client can send", e.method);
        }
        if let Some(call) = bot.answer(&e) {
            if bot.holds && e.method == "GAME_SELECT" && matches!(call, Call::Uuid(Some(_))) {
                game.call(Call::Action("HOLD_PRIORITY".into(), None));
            }
            game.call(call);
        }
        assert!(events.len() < 50_000, "the game doesn't end");
    }
    (game.join(), events)
}

fn start(human_deck: &Deck, ai_deck: &Deck, seed: i64, chooser: Option<&str>) -> Start {
    Start {
        game_id: format!("game-{seed}"),
        seed,
        seats: vec![
            SeatStart {
                player_id: HUMAN.into(),
                name: "tester".into(),
                human: true,
                deck: human_deck.cards.clone(),
                printings: Vec::new(),
                user_data: None,
            },
            SeatStart {
                player_id: AI.into(),
                name: "Computer".into(),
                human: false,
                deck: ai_deck.cards.clone(),
                printings: Vec::new(),
                user_data: None,
            },
        ],
        timer: Timer::default(),
        chooser: chooser.map(str::to_string),
        record: None,
        log_to: None,
        drop_stale: false,
        replay: None,
        rules: playmat_host::TableRules::default(),
        prelude: None,
        lines: false,
    }
}

/// The game's first question after the join, and the log lines before it
fn first_question(game: &HostedGame) -> (Event, Vec<String>) {
    let mut log = Vec::new();
    loop {
        let e = game.events.recv_timeout(Duration::from_secs(60)).expect("the host answers within a minute");
        match e.method.as_str() {
            "GAME_INIT" | "GAME_UPDATE" => {}
            "CHATMESSAGE" => log.push(e.data["message"].as_str().unwrap_or("").to_string()),
            _ => return (e, log),
        }
    }
}

#[test]
fn games_end_with_game_over() {
    let company = deck("gw-company.txt");
    let zombies = deck("b-zombies.txt");
    for seed in 0..4 {
        let (mine, theirs) = if seed % 2 == 0 { (&company, &zombies) } else { (&zombies, &company) };
        let (ended, events) = play(mine, theirs, seed, u64::MAX);
        assert!(events[0].method == "GAME_INIT", "first {}", events[0].method);
        let prompts = events.iter().filter(|e| e.method.starts_with("GAME_") && e.method != "GAME_UPDATE").count();
        let log = events.iter().filter(|e| e.method == "CHATMESSAGE").count();
        println!("seed {seed}: {:?} after {} turns, {prompts} prompts, {log} log lines", ended.winner, ended.turns);
        assert!(ended.reason == "over", "seed {seed}: {}", ended.reason);
        assert!(ended.turns > 2);
        // the client never plays a spell: the AI wins
        assert_eq!(ended.winner, Some(1), "seed {seed}");
    }
}

#[test]
fn a_concede_ends_the_game() {
    let (ended, events) = play(&deck("gw-company.txt"), &deck("b-zombies.txt"), 7, 3);
    assert_eq!(ended.reason, "conceded");
    assert_eq!(ended.winner, Some(1));
    assert_eq!(events.last().unwrap().method, "GAME_OVER");
}

#[test]
fn the_priority_timer_runs_out() {
    let timer = Timer { priority_secs: 2, buffer_secs: 1, ..Timer::default() };
    let (ended, events) = play_timed(&deck("gw-company.txt"), &deck("b-zombies.txt"), 3, u64::MAX, timer, true);
    assert_eq!(ended.reason, "timed out");
    // the human lost
    assert_eq!(ended.winner, Some(1));
    // the time left goes to the match's next game: none for the human, the AI's untouched or nearly
    assert_eq!(ended.time_left[0], Some(0));
    assert!(ended.time_left[1].is_some());
    let over = events.last().unwrap();
    assert_eq!(over.method, "GAME_OVER");
    let log: Vec<&str> =
        events.iter().filter(|e| e.method == "CHATMESSAGE").filter_map(|e| e.data["message"].as_str()).collect();
    assert!(log.iter().any(|l| l.contains("tester</font> has run out of time, losing the match.")), "{log:?}");
    // the views carry Java's timer fields: the game's settings, each player's time left and whose clock runs
    let select = events.iter().find(|e| e.method == "GAME_SELECT").expect("a priority prompt");
    let view = &select.data["gameView"];
    assert_eq!(view["priorityTime"], 2);
    assert_eq!(view["bufferTime"], 1);
    let me = view["players"].as_array().unwrap().iter().find(|p| p["isHuman"] == true).unwrap();
    assert!(me["priorityTimeLeftSecs"].as_i64().unwrap() <= 2);
    assert_eq!(me["bufferTimeLeft"], 1);
    assert_eq!(me["timerActive"], true);
    assert!(me["priorityTimeSavedTimeMs"].as_u64().unwrap() > 0);
    let me =
        over.data["gameView"]["players"].as_array().unwrap().iter().find(|p| p["isHuman"] == true).unwrap().clone();
    assert_eq!(me["priorityTimeLeftSecs"], 0);
}

#[test]
fn no_timer_views() {
    let (_, events) = play(&deck("gw-company.txt"), &deck("b-zombies.txt"), 7, 3);
    let view = &events.iter().find(|e| e.method == "GAME_SELECT").unwrap().data["gameView"];
    assert_eq!(view["priorityTime"], 0);
    for p in view["players"].as_array().unwrap() {
        // Java's Integer.MAX_VALUE: no limit
        assert_eq!(p["priorityTimeLeftSecs"], i32::MAX as i64);
        assert_eq!(p["bufferTimeLeft"], 0);
    }
}

/// A match's next game: the last game's loser chooses who plays first (`MatchImpl.getChooser`)
#[test]
fn the_chooser_picks_who_plays_first() {
    let (company, zombies) = (deck("gw-company.txt"), deck("b-zombies.txt"));
    let db = Arc::new(playmat_engine::cards());
    for seed in 0..3 {
        // the human lost: they're asked who starts
        let game = HostedGame::start(start(&company, &zombies, seed, Some(HUMAN)), db.clone()).unwrap();
        game.call(Call::Join);
        let (e, _) = first_question(&game);
        assert_eq!(e.data["options"]["webPrompt"], "startingPlayer", "seed {seed}: {}", e.method);
        game.call(Call::Quit);
        let ended = game.join();
        assert_eq!(ended.reason, "quit");
        assert_eq!(ended.winner, Some(1));
        // the AI lost: it takes the first turn, and the human is asked about their hand
        let game = HostedGame::start(start(&company, &zombies, seed, Some(AI)), db.clone()).unwrap();
        game.call(Call::Join);
        let (e, log) = first_question(&game);
        assert_ne!(e.data["options"]["webPrompt"], "startingPlayer", "seed {seed}");
        assert!(log.iter().any(|l| l.contains("Computer") && l.contains("first turn")), "seed {seed}: {log:?}");
        game.call(Call::Action("CONCEDE".into(), None));
        assert_eq!(game.join().reason, "conceded");
    }
}

/// A match's later game starts each clock where the earlier games left it (`MatchImpl.initGame`)
#[test]
fn a_match_keeps_the_time_left() {
    let (company, zombies) = (deck("gw-company.txt"), deck("b-zombies.txt"));
    let timer = Timer { priority_secs: 600, buffer_secs: 0, left: [Some(42), None] };
    let start = Start { timer, ..start(&company, &zombies, 1, Some(HUMAN)) };
    let game = HostedGame::start(start, Arc::new(playmat_engine::cards())).unwrap();
    game.call(Call::Join);
    let (e, _) = first_question(&game);
    let players = e.data["gameView"]["players"].as_array().unwrap().clone();
    let left = |human: bool| players.iter().find(|p| p["isHuman"] == human).unwrap()["priorityTimeLeftSecs"].clone();
    assert!(left(true).as_i64().unwrap() <= 42 && left(true).as_i64().unwrap() >= 41, "{}", left(true));
    assert_eq!(left(false), 600);
    game.call(Call::Action("CONCEDE".into(), None));
    let ended = game.join();
    assert!(ended.time_left[0].is_some_and(|l| l <= 42));
    assert_eq!(ended.time_left[1], Some(600));
}

/// A recorded game plays the same again from its record: the same log, the same end
#[test]
fn a_record_replays_the_game() {
    let (company, zombies) = (deck("gw-company.txt"), deck("b-zombies.txt"));
    let path = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("host-records/game-7.jsonl");
    let start = Start { record: Some(path.clone()), ..start(&company, &zombies, 7, None) };
    let (ended, _) = play_start(start, 6, false);
    let rec = playmat_host::record::read(&path).unwrap();
    assert!(!rec.calls.is_empty() && !rec.log.is_empty());
    assert_eq!(rec.end.as_ref().unwrap()["reason"], ended.reason);
    let replayed = playmat_host::record::replay(&rec, Arc::new(playmat_engine::cards())).unwrap();
    assert_eq!(replayed.first_difference, None, "the replay's log differs from the record's");
    assert_eq!((replayed.ended.winner, replayed.ended.turns), (ended.winner, ended.turns));
    assert_eq!(replayed.ended.reason, ended.reason);
}

/// The views carry the game's events (`GameEventView`) for the client's effects: combat damage to the human, each
/// draw (the AI's without its card), in order; and a spell's `cardId` is its card's id, never "?"
#[test]
fn players_carry_their_avatar() {
    let mut s = start(&deck("gw-company.txt"), &deck("b-zombies.txt"), 3, None);
    s.seats[0].user_data = Some(serde_json::json!({"avatarId": 12, "passPriorityCast": false}));
    let game = HostedGame::start(s, Arc::new(playmat_engine::cards())).unwrap();
    game.call(Call::Join);
    let init = loop {
        let e = game.events.recv_timeout(Duration::from_secs(60)).expect("the host answers");
        if e.method == "GAME_INIT" {
            break e;
        }
    };
    assert_eq!(init.data["players"][0]["userData"]["avatarId"], 12);
    assert_eq!(init.data["players"][1]["userData"]["avatarId"], 0, "the AI's is the default");
    game.call(Call::Action("CONCEDE".into(), None));
}

#[test]
fn views_carry_the_games_events() {
    let (_, events) = play(&deck("gw-company.txt"), &deck("b-zombies.txt"), 0, u64::MAX);
    let mut seen = Vec::new();
    for e in &events {
        let view = if e.method == "GAME_UPDATE" || e.method == "GAME_INIT" { &e.data } else { &e.data["gameView"] };
        seen.extend(view["events"].as_array().cloned().unwrap_or_default());
        for card in view["stack"].as_object().into_iter().flat_map(|s| s.values()) {
            assert_ne!(card["cardId"], "?", "a spell's cardId");
        }
    }
    let seqs: Vec<u64> = seen.iter().map(|e| e["seq"].as_u64().unwrap()).collect();
    assert!(seqs.windows(2).all(|w| w[0] < w[1]), "events in order, each once");
    assert!(
        seen.iter().any(|e| e["kind"] == "DAMAGE"
            && e["combat"] == true
            && e["targetId"] == HUMAN
            && e["sourceId"].as_str().is_some_and(uuid)),
        "combat damage to the human"
    );
    let draws = |p: &'static str| seen.iter().filter(move |e| e["kind"] == "DRAW" && e["playerId"] == p);
    assert!(draws(HUMAN).count() > 0 && draws(HUMAN).all(|e| e["targetId"].as_str().is_some_and(uuid)));
    assert!(draws(AI).count() > 0 && draws(AI).all(|e| e.get("targetId").is_none()), "the AI's draws are hidden");
    assert!(seen.iter().any(|e| e["kind"] == "LIFE_LOSS" && e["playerId"] == HUMAN));
}

#[test]
fn the_end_carries_the_humans_career_tally() {
    let (ended, _) = play(&deck("gw-company.txt"), &deck("b-zombies.txt"), 0, u64::MAX);
    let tally = &ended.summary["tally"];
    let counters = &tally["counters"];
    let n = |k: &str| counters[k].as_i64().unwrap_or(0);
    assert!(n("my_turns") > 0, "turns counted: {tally}");
    assert!(n("lands_played") > 0, "lands counted: {tally}");
    assert!(n("cards_drawn") > 0, "draws counted: {tally}");
    // (the test's player casts little and loses to the damage)
    let life = tally["life"].as_i64().unwrap();
    assert_eq!(n("damage_taken"), 20 - life + n("life_gained"), "damage counted: {tally}");
    assert_eq!(tally["startingLife"], 20);
    assert_eq!(ended.summary["players"].as_array().map(Vec::len), Some(2));
    assert_eq!(tally["life"], ended.summary["players"][0]["life"], "the human's life, seat 0");
    assert!(tally["lowestLife"].as_i64().unwrap() <= tally["life"].as_i64().unwrap());
}

#[test]
fn a_tables_starting_life_and_free_mulligans_are_played() {
    let mut s = start(&deck("gw-company.txt"), &deck("b-zombies.txt"), 3, None);
    s.rules = playmat_host::TableRules { starting_life: 30, free_mulligans: 1, ai_skill: Some(3), rollback: false };
    let game = HostedGame::start(s, Arc::new(playmat_engine::cards())).unwrap();
    game.call(Call::Join);
    let mut asked = 0;
    let mut log = Vec::new();
    loop {
        let e = game.events.recv_timeout(Duration::from_secs(60)).expect("the host answers");
        if e.method == "CHATMESSAGE" {
            log.push(e.data["message"].as_str().unwrap_or("").to_string());
        }
        if e.method == "GAME_INIT" {
            assert_eq!(e.data["players"][0]["life"], 30, "the table's starting life");
        }
        if e.method == "GAME_ASK" {
            // a mulligan: once for free, then keep
            asked += 1;
            let size_text = e.data["message"].as_str().unwrap_or("").to_string();
            let size = if asked == 1 { "7" } else { "6" };
            assert!(size_text.contains(size), "mulligan {asked} goes down to {size}: {size_text}");
            game.call(Call::Bool(asked == 1));
            if asked == 2 {
                break;
            }
        }
    }
    assert!(log.iter().any(|l| l.contains("mulligans for free")), "{log:?}");
    game.call(Call::Action("CONCEDE".into(), None));
}

#[test]
fn a_hosted_game_writes_its_replay() {
    use std::io::BufRead;
    let path = std::env::temp_dir().join(format!("playmat-replay-{}.ndjson.gz.part", std::process::id()));
    let s = Start { replay: Some(path.clone()), ..start(&deck("gw-company.txt"), &deck("b-zombies.txt"), 0, None) };
    let (ended, _) = play_start(s, u64::MAX, false);
    let events = ended.summary["replayEvents"].as_u64().unwrap();
    let file = std::fs::File::open(&path).unwrap();
    let lines: Vec<serde_json::Value> = std::io::BufReader::new(flate2::read::GzDecoder::new(file))
        .lines()
        .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
        .collect();
    std::fs::remove_file(&path).ok();
    assert_eq!(lines.len() as u64, events);
    let frames: Vec<_> = lines.iter().filter(|l| l.get("view").is_some()).collect();
    assert!(frames.len() > 10, "board snapshots");
    for f in &frames {
        assert!(f["view"].get("myHand").is_none(), "a spectator's view");
        assert!(f["hands"]["tester"].is_object(), "the human's hand beside it");
        assert!(f["t"].is_u64());
    }
    assert!(lines.iter().any(|l| l["log"]["message"].is_string()), "the game log");
    assert!(frames.windows(2).all(|w| w[0]["view"] != w[1]["view"]), "no snapshot twice");
}

/// Priority questions the human was asked with only their own spells and abilities on the stack
fn asked_over_own_stack(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|e| e.method == "GAME_SELECT")
        .filter(|e| {
            let stack = e.data["gameView"]["stack"].as_object().cloned().unwrap_or_default();
            !stack.is_empty() && stack.values().all(|c| c["controllerId"] == HUMAN)
        })
        .count()
}

#[test]
fn pass_priority_after_casting_passes_unless_held() {
    let deck_h = deck("gw-company.txt");
    let deck_a = deck("b-zombies.txt");
    let mut total = (0, 0, 0);
    for seed in 0..4 {
        let caster =
            || Bot { tried: HashSet::new(), turn: 0, concede_at: 12, silent: false, casts: true, holds: false };
        let (_, plain) = play_with(start(&deck_h, &deck_a, seed, None), caster());
        let mut s = start(&deck_h, &deck_a, seed, None);
        s.seats[0].user_data = Some(serde_json::json!({"passPriorityCast": true, "passPriorityActivation": true}));
        let (_, passing) = play_with(s.clone(), caster());
        let (_, held) = play_with(s, Bot { holds: true, ..caster() });
        total.0 += asked_over_own_stack(&plain);
        total.1 += asked_over_own_stack(&passing);
        total.2 += asked_over_own_stack(&held);
    }
    assert!(total.0 > 0, "the player is asked again after casting by default");
    assert_eq!(total.1, 0, "never with pass priority after casting on (asked {} times without it)", total.0);
    assert!(total.2 > 0, "holding priority asks again");
}

/// The human's untapped lands in a question's view
fn my_untapped_lands(view: &Value) -> Vec<String> {
    let me = view["myPlayerId"].as_str().unwrap_or("");
    view["players"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["playerId"] == me)
        .flat_map(|p| {
            p["battlefield"]
                .as_object()
                .into_iter()
                .flatten()
                .filter(|(_, c)| {
                    c["tapped"] != true && c["cardTypes"].as_array().is_some_and(|t| t.iter().any(|x| x == "LAND"))
                })
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn my_pool(view: &Value) -> i64 {
    let me = view["myPlayerId"].as_str().unwrap_or("");
    let p = view["players"].as_array().unwrap().iter().find(|p| p["playerId"] == me).unwrap();
    ["red", "green", "blue", "white", "black", "colorless"].iter().map(|c| p["manaPool"][c].as_i64().unwrap_or(0)).sum()
}

fn me_of(view: &Value) -> Value {
    let me = view["myPlayerId"].as_str().unwrap_or("").to_string();
    view["players"].as_array().unwrap().iter().find(|p| p["playerId"] == me.as_str()).unwrap().clone()
}

#[test]
fn undo_tap_and_rollback_take_the_game_back() {
    let mut s = start(&deck("gw-company.txt"), &deck("b-zombies.txt"), 2, None);
    s.rules.rollback = true;
    let game = HostedGame::start(s, Arc::new(playmat_engine::cards())).unwrap();
    game.call(Call::Join);
    let mut bot =
        Bot { tried: HashSet::new(), turn: 0, concede_at: u64::MAX, silent: false, casts: false, holds: false };
    // 0: playing to a main phase of the human's with an untapped land; 1: tapped it; 2: undone; 3: rolled back
    let mut stage = 0;
    let mut turn_at = 0;
    let mut log = Vec::new();
    for _ in 0..20_000 {
        let e = game.events.recv_timeout(Duration::from_secs(60)).expect("the host answers");
        if e.method == "CHATMESSAGE" {
            log.push(e.data["message"].as_str().unwrap_or("").to_string());
        }
        assert_ne!(e.method, "GAME_OVER", "the test is done before the game is");
        let v = &e.data["gameView"];
        let priority = e.method == "GAME_SELECT" && e.data["message"].as_str().is_some_and(|m| m.starts_with("Play"));
        if !priority {
            if let Some(c) = bot.answer(&e) {
                game.call(c);
            }
            continue;
        }
        let mine = v["activePlayerId"] == v["myPlayerId"];
        let main = v["step"].as_str().is_some_and(|s| s.contains("MAIN"));
        let empty = v["stack"].as_object().is_none_or(|m| m.is_empty());
        match stage {
            0 if mine && main && empty && v["turn"].as_u64().unwrap() >= 3 && !my_untapped_lands(v).is_empty() => {
                assert_eq!(me_of(v)["statesSavedSize"], 0);
                assert_eq!(v["rollbackTurnsAllowed"], true);
                stage = 1;
                game.call(Call::Uuid(Some(my_untapped_lands(v)[0].clone())));
            }
            1 => {
                assert!(my_pool(v) > 0, "the land made mana");
                assert_eq!(me_of(v)["statesSavedSize"], 1, "it can be undone");
                stage = 2;
                game.call(Call::Action("UNDO".into(), None));
            }
            2 => {
                assert_eq!(my_pool(v), 0, "the mana is gone");
                assert_eq!(me_of(v)["statesSavedSize"], 0);
                turn_at = v["turn"].as_u64().unwrap();
                stage = 3;
                game.call(Call::Action("ROLLBACK_TURNS".into(), Some(serde_json::json!(0))));
            }
            3 => {
                assert_eq!(v["turn"].as_u64().unwrap(), turn_at, "back to the start of the turn");
                assert!(log.iter().any(|l| l.contains(&format!("Rolling back to start of turn {turn_at}"))), "{log:?}");
                game.call(Call::Action("CONCEDE".into(), None));
                return;
            }
            _ => {
                if let Some(c) = bot.answer(&e) {
                    game.call(c);
                }
            }
        }
    }
    panic!("never got to stage 3 (stage {stage})");
}

#[test]
fn a_late_answer_is_dropped_not_given_to_the_next_question() {
    let mut s = start(&deck("gw-company.txt"), &deck("b-zombies.txt"), 3, None);
    s.drop_stale = true;
    let game = HostedGame::start(s, Arc::new(playmat_engine::cards())).unwrap();
    game.call(Call::Join);
    // the questions asked: an answer that doesn't fit one would ask it again
    let mut asked = Vec::new();
    loop {
        let wait = Duration::from_millis(if asked.len() < 2 { 60_000 } else { 1_500 });
        let Ok(e) = game.events.recv_timeout(wait) else { break };
        if matches!(e.method.as_str(), "GAME_INIT" | "GAME_UPDATE" | "CHATMESSAGE") {
            continue;
        }
        asked.push(e.method.clone());
        if asked.len() == 1 {
            // a double click on "Mulligan": the second is for the question that's gone
            game.call(Call::Bool(true));
            game.call(Call::Bool(true));
        }
    }
    assert_eq!(asked, ["GAME_ASK", "GAME_TARGET"], "the card to put on the bottom is asked once");
    game.call(Call::Action("CONCEDE".into(), None));
}

const OTHER: &str = "33333333-3333-4333-8333-333333333333";

/// Two people: both seats human, each its own player id and name
fn people(a: &Deck, b: &Deck, seed: i64) -> Start {
    let mut s = start(a, b, seed, None);
    s.seats[1] = SeatStart {
        player_id: OTHER.into(),
        name: "other".into(),
        human: true,
        deck: b.cards.clone(),
        printings: Vec::new(),
        user_data: None,
    };
    s
}

fn bot(concede_at: u64) -> Bot {
    Bot { tried: HashSet::new(), turn: 0, concede_at, silent: false, casts: true, holds: false }
}

/// A game between two people plays to its end: each seat gets its own board and questions, never the other's hand
#[test]
fn two_people_play_a_game_to_its_end() {
    let path = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("host-records/people-5.jsonl");
    let s = Start { record: Some(path.clone()), ..people(&deck("gw-company.txt"), &deck("b-zombies.txt"), 5) };
    let game = HostedGame::start(s, Arc::new(playmat_engine::cards())).unwrap();
    game.call_as(0, Call::Join);
    game.call_as(1, Call::Join);
    let mut bots = [bot(u64::MAX), bot(u64::MAX)];
    let ids = [HUMAN, OTHER];
    let mut over = [false, false];
    let mut asked = [0, 0];
    let mut log = 0;
    for _ in 0..100_000 {
        let e = game.events.recv_timeout(Duration::from_secs(60)).expect("the host answers within a minute");
        let Some(seat) = e.to else {
            assert_eq!(e.method, "CHATMESSAGE", "only the log goes to both");
            log += 1;
            continue;
        };
        let view = if matches!(e.method.as_str(), "GAME_INIT" | "GAME_UPDATE") { &e.data } else { &e.data["gameView"] };
        if view.is_object() {
            assert_eq!(view["myPlayerId"], ids[seat], "{}'s board is theirs", ids[seat]);
            let players = view["players"].as_array().unwrap();
            assert!(players.iter().all(|p| p["isHuman"] == true), "both players are human");
        }
        if e.method == "GAME_OVER" {
            over[seat] = true;
            if over == [true, true] {
                break;
            }
            continue;
        }
        if let Some(c) = bots[seat].answer(&e) {
            asked[seat] += 1;
            game.call_as(seat, c);
        }
    }
    let ended = game.join();
    assert_eq!(over, [true, true], "both players see the end");
    assert!(asked[0] > 10 && asked[1] > 10, "both were asked: {asked:?}");
    assert!(log > 0);
    assert_eq!(ended.reason, "over");
    assert!(ended.winner.is_some());
    let tallies = ended.summary["tallies"].as_object().unwrap();
    assert!(tallies.contains_key(HUMAN) && tallies.contains_key(OTHER), "each player's tally");
    // its record plays the same game again, each call from its seat
    let rec = playmat_host::record::read(&path).unwrap();
    assert!(rec.calls.iter().any(|(s, _)| *s == 1) && rec.calls.iter().any(|(s, _)| *s == 0));
    let replayed = playmat_host::record::replay(&rec, Arc::new(playmat_engine::cards())).unwrap();
    assert_eq!(replayed.first_difference, None, "the replay's log differs from the record's");
    assert_eq!((replayed.ended.winner, replayed.ended.turns), (ended.winner, ended.turns));
}

/// Between people, a concede or a quit from the player who isn't being asked ends the game as their loss
#[test]
fn a_player_not_asked_can_concede() {
    let game = HostedGame::start(
        people(&deck("gw-company.txt"), &deck("b-zombies.txt"), 6),
        Arc::new(playmat_engine::cards()),
    )
    .unwrap();
    game.call_as(0, Call::Join);
    game.call_as(1, Call::Join);
    // the first question goes to one of them; the other concedes
    let asked = loop {
        let e = game.events.recv_timeout(Duration::from_secs(60)).expect("the host answers");
        if !matches!(e.method.as_str(), "GAME_INIT" | "GAME_UPDATE" | "CHATMESSAGE") {
            break e.to.unwrap();
        }
    };
    game.call_as(1 - asked, Call::Quit);
    while let Ok(e) = game.events.recv_timeout(Duration::from_secs(60)) {
        if e.method == "GAME_OVER" && e.to == Some(asked) {
            break;
        }
    }
    let ended = game.join();
    assert_eq!(ended.winner, Some(asked));
    assert_eq!(ended.reason, "quit");
}

/// Between people a rollback needs the other player's consent (`GameController.requestPermissionToRollback`)
#[test]
fn a_rollback_between_people_waits_for_the_others_consent() {
    let mut s = people(&deck("gw-company.txt"), &deck("b-zombies.txt"), 2);
    s.rules.rollback = true;
    let game = HostedGame::start(s, Arc::new(playmat_engine::cards())).unwrap();
    game.call_as(0, Call::Join);
    game.call_as(1, Call::Join);
    let mut bots = [bot(u64::MAX), bot(u64::MAX)];
    let mut asked_for = None;
    let mut log = Vec::new();
    for _ in 0..50_000 {
        let e = game.events.recv_timeout(Duration::from_secs(60)).expect("the host answers");
        if e.method == "CHATMESSAGE" {
            log.push(e.data["message"].as_str().unwrap_or("").to_string());
            if log.iter().any(|l| l.contains("Rolling back to start of turn 3")) {
                game.call_as(0, Call::Action("CONCEDE".into(), None));
                return;
            }
            continue;
        }
        assert_ne!(e.method, "GAME_OVER", "the rollback comes before the end");
        let Some(seat) = e.to else { continue };
        if e.method == "USER_REQUEST_DIALOG" {
            assert_eq!(Some(1 - seat), asked_for, "the other player is asked");
            assert_eq!(e.data["button1Action"], "ADD_PERMISSION_TO_ROLLBACK_TURN");
            game.call_as(seat, Call::Action("ADD_PERMISSION_TO_ROLLBACK_TURN".into(), None));
            continue;
        }
        let v = &e.data["gameView"];
        let priority = e.method == "GAME_SELECT" && e.data["message"].as_str().is_some_and(|m| m.starts_with("Play"));
        if asked_for.is_none() && priority && v["turn"] == 3 {
            asked_for = Some(seat);
            game.call_as(seat, Call::Action("ROLLBACK_TURNS".into(), Some(serde_json::json!(0))));
            continue;
        }
        if let Some(c) = bots[seat].answer(&e) {
            game.call_as(seat, c);
        }
    }
    panic!("no rollback: {log:?}");
}

/// What a seat holds and what's on the battlefield, by card name, as a view shows it
fn board(view: &Value) -> (Vec<String>, Vec<String>, Vec<i64>) {
    let names = |m: &Value| {
        let mut v: Vec<String> =
            m.as_object().into_iter().flatten().map(|(_, c)| c["name"].as_str().unwrap_or("").to_string()).collect();
        v.sort();
        v
    };
    let players = view["players"].as_array().unwrap();
    let field: Vec<String> = players.iter().flat_map(|p| names(&p["battlefield"])).collect();
    (names(&view["myHand"]), field, players.iter().map(|p| p["life"].as_i64().unwrap()).collect())
}

/// A game's line (`HOST_TURN`) carries it on in another game from that turn's start: the same board, the same cards
/// to come, played to its end (a save, "Try this turn again", a what-if)
#[test]
fn a_line_carries_the_game_on_from_a_turns_start() {
    let (company, zombies) = (deck("gw-company.txt"), deck("b-zombies.txt"));
    let mut s = start(&company, &zombies, 4, None);
    s.lines = true;
    s.rules.rollback = true;
    let game = HostedGame::start(s, Arc::new(playmat_engine::cards())).unwrap();
    game.call(Call::Join);
    let mut b = bot(u64::MAX);
    let mut line = None;
    let mut turns = Vec::new();
    let saved = loop {
        let e = game.events.recv_timeout(Duration::from_secs(60)).expect("the host answers");
        assert_ne!(e.method, "GAME_OVER");
        if e.method == "HOST_TURN" {
            assert_eq!(e.to, None);
            turns.push(e.data["turn"].as_u64().unwrap());
            if e.data["turn"] == 4 {
                line = Some(e.data["line"].clone());
            }
            continue;
        }
        let view = if matches!(e.method.as_str(), "GAME_INIT" | "GAME_UPDATE") { &e.data } else { &e.data["gameView"] };
        // the board at turn 4's start, before anyone acted in it
        if line.is_some() && view.is_object() {
            break board(view);
        }
        if let Some(c) = b.answer(&e) {
            game.call(c);
        }
    };
    game.call(Call::Action("CONCEDE".into(), None));
    let _ = game.join();
    assert_eq!(turns[..4], [1, 2, 3, 4], "a line at each turn's start");
    let line = line.unwrap();
    let prelude = playmat_host::record::prelude_of(&line).expect("the line reads back");
    assert_eq!(prelude.turn, 4);
    // a new table: its own game and player ids, the same seats and decks
    let mut s = start(&company, &zombies, 99, None);
    s.game_id = "game-carried-on".into();
    s.seats[0].player_id = "44444444-4444-4444-8444-444444444444".into();
    s.seats[1].player_id = "55555555-5555-4555-8555-555555555555".into();
    s.prelude = Some(prelude);
    s.lines = true;
    let path = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("host-records/carried-on.jsonl");
    s.record = Some(path.clone());
    let game = HostedGame::start(s, Arc::new(playmat_engine::cards())).unwrap();
    game.call(Call::Join);
    let mut first = None;
    let mut log = Vec::new();
    let mut b = bot(8);
    loop {
        let e = game.events.recv_timeout(Duration::from_secs(60)).expect("the carried-on game answers");
        if e.method == "CHATMESSAGE" {
            log.push(e.data["message"].as_str().unwrap_or("").to_string());
        }
        if e.method == "GAME_INIT" {
            assert_eq!(e.data["turn"], 4, "the game carries on at turn 4");
            assert_eq!(e.data["myPlayerId"], "44444444-4444-4444-8444-444444444444", "with the new table's ids");
            first = Some(board(&e.data));
        }
        if e.method == "GAME_OVER" {
            break;
        }
        if let Some(c) = b.answer(&e) {
            game.call(c);
        }
    }
    let ended = game.join();
    assert_eq!(first.as_ref(), Some(&saved), "the same board as at the turn's start");
    assert!(log.iter().any(|l| l.contains("carries on from the start of turn 4")), "{log:?}");
    assert!(!log.iter().any(|l| l.contains("keeps hand")), "the earlier game's log isn't sent again");
    assert!(ended.turns >= 4);
    // the carried-on game's record (its start holds the line) plays it again too
    let rec = playmat_host::record::read(&path).unwrap();
    assert!(rec.start.prelude.is_some());
    let replayed = playmat_host::record::replay(&rec, Arc::new(playmat_engine::cards())).unwrap();
    assert_eq!(replayed.first_difference, None, "the replay's log differs from the record's");
    assert_eq!((replayed.ended.winner, replayed.ended.turns), (ended.winner, ended.turns));
}

/// Practice tools and advice against the AI: life set, a card into the hand, and the AI's pick at a question
#[test]
fn practice_tools_and_advice_against_the_ai() {
    let game = HostedGame::start(
        start(&deck("gw-company.txt"), &deck("b-zombies.txt"), 3, None),
        Arc::new(playmat_engine::cards()),
    )
    .unwrap();
    game.call(Call::Join);
    let mut b = bot(u64::MAX);
    let mut stage = 0;
    let mut log = Vec::new();
    for _ in 0..20_000 {
        let e = game.events.recv_timeout(Duration::from_secs(60)).expect("the host answers");
        if e.method == "CHATMESSAGE" {
            log.push(e.data["message"].as_str().unwrap_or("").to_string());
            continue;
        }
        assert_ne!(e.method, "GAME_OVER");
        let v = &e.data["gameView"];
        let priority = e.method == "GAME_SELECT" && e.data["message"].as_str().is_some_and(|m| m.starts_with("Play"));
        if !priority || v["turn"].as_u64().unwrap_or(0) < 2 {
            if let Some(c) = b.answer(&e) {
                game.call(c);
            }
            continue;
        }
        match stage {
            0 => {
                let (tx, rx) = std::sync::mpsc::channel();
                game.call(Call::Advise(4, tx));
                let advice = rx.recv_timeout(Duration::from_secs(30)).expect("advice");
                assert!(advice["kind"].as_str().is_some_and(|k| k != "NONE"), "{advice}");
                assert!(advice["text"].as_str().is_some_and(|t| !t.is_empty()));
                let (tx, rx) = std::sync::mpsc::channel();
                game.call(Call::Practice("LIFE".into(), None, 7, Some(tx)));
                assert_eq!(rx.recv_timeout(Duration::from_secs(30)).unwrap(), "");
                stage = 1;
            }
            1 => {
                assert_eq!(me_of(v)["life"], 7, "the life the tool set");
                assert!(log.iter().any(|l| l.contains("Practice:") && l.contains("sets their life to 7")), "{log:?}");
                let hand = v["myHand"].as_object().unwrap().len();
                let (tx, rx) = std::sync::mpsc::channel();
                game.call(Call::Practice("HAND".into(), Some("Llanowar Elves".into()), 2, Some(tx)));
                assert_eq!(rx.recv_timeout(Duration::from_secs(30)).unwrap(), "");
                stage = 2;
                // (checked at the next question)
                let _ = hand;
            }
            2 => {
                let elves = v["myHand"].as_object().unwrap().values().filter(|c| c["name"] == "Llanowar Elves").count();
                assert!(elves >= 2, "two more Llanowar Elves in hand");
                let (tx, rx) = std::sync::mpsc::channel();
                game.call(Call::Practice("TELEPORT".into(), None, 1, Some(tx)));
                assert!(rx.recv_timeout(Duration::from_secs(30)).unwrap().contains("Unknown"));
                game.call(Call::Action("CONCEDE".into(), None));
                return;
            }
            _ => unreachable!(),
        }
    }
    panic!("stopped at stage {stage}");
}
