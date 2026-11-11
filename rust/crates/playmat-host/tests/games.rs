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
                            if self.casts && playable.is_some_and(|p| p.contains_key(id)) && self.tried.insert(id.clone()) {
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
        replay: None,
        rules: playmat_host::TableRules::default(),
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
    s.rules = playmat_host::TableRules { starting_life: 30, free_mulligans: 1, ai_skill: Some(3) };
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
        let caster = || Bot { tried: HashSet::new(), turn: 0, concede_at: 12, silent: false, casts: true, holds: false };
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
