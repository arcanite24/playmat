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
}

impl Bot {
    fn answer(&mut self, e: &Event) -> Option<Call> {
        let d = &e.data;
        let view = &d["gameView"];
        let turn = view["turn"].as_u64().unwrap_or(0);
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
            "GAME_PLAY_MANA" | "GAME_PLAY_XMANA" => Call::Bool(false),
            _ => return None,
        })
    }
}

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
    let db = Arc::new(playmat_engine::cards());
    let start = Start {
        game_id: format!("game-{seed}"),
        seed,
        seats: vec![
            SeatStart {
                player_id: "11111111-1111-4111-8111-111111111111".into(),
                name: "tester".into(),
                human: true,
                deck: human_deck.cards.clone(),
                user_data: None,
            },
            SeatStart {
                player_id: "22222222-2222-4222-8222-222222222222".into(),
                name: "Computer".into(),
                human: false,
                deck: ai_deck.cards.clone(),
                user_data: None,
            },
        ],
        timer,
    };
    let game = HostedGame::start(start, db).unwrap();
    game.call(Call::Join);
    let mut bot = Bot { tried: HashSet::new(), turn: 0, concede_at, silent };
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
            game.call(call);
        }
        assert!(events.len() < 50_000, "the game doesn't end");
    }
    (game.join(), events)
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
    let timer = Timer { priority_secs: 2, buffer_secs: 1 };
    let (ended, events) = play_timed(&deck("gw-company.txt"), &deck("b-zombies.txt"), 3, u64::MAX, timer, true);
    assert_eq!(ended.reason, "timed out");
    // the human lost
    assert_eq!(ended.winner, Some(1));
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
