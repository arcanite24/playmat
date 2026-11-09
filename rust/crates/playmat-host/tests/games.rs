//! Whole games on the host: a scripted client (plays lands, never casts, keeps its hand) against the Rust AI,
//! through the client's protocol only. Every game must end with `GAME_OVER`, and every id the client gets must be
//! one it can send back.

use playmat_ai::duel::Deck;
use playmat_host::{Call, Event, HostedGame, SeatStart, Start};
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

const HUMAN: &str = "11111111-1111-4111-8111-111111111111";
const AI: &str = "22222222-2222-4222-8222-222222222222";

fn play(human_deck: &Deck, ai_deck: &Deck, seed: i64, concede_at: u64) -> (playmat_host::Ended, Vec<Event>) {
    let game = HostedGame::start(start(human_deck, ai_deck, seed, None), Arc::new(playmat_engine::cards())).unwrap();
    game.call(Call::Join);
    let mut bot = Bot { tried: HashSet::new(), turn: 0, concede_at };
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
                user_data: None,
            },
            SeatStart {
                player_id: AI.into(),
                name: "Computer".into(),
                human: false,
                deck: ai_deck.cards.clone(),
                user_data: None,
            },
        ],
        chooser: chooser.map(str::to_string),
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
