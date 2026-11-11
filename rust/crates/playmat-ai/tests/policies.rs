//! The one-decision interface (`decision::DecisionPolicy`): each shipped policy plays whole games, a policy set on a
//! table answers its seat, and MCTS plays the same game twice from the same seed.

use playmat_ai::decision::{Choice, Decision, DecisionPolicy, PolicyKind};
use playmat_ai::duel::{new_duel, Deck};
use playmat_ai::search::{Config, Table};
use playmat_engine::rules;
use playmat_engine::state::Game;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn deck(file: &str) -> Deck {
    let path = format!("{}/../playmat-engine/shadow-decks/tier30/{file}", env!("CARGO_MANIFEST_DIR"));
    Deck::parse(file, &std::fs::read_to_string(path).unwrap())
}

fn play(configs: [Option<Config>; 2], seed: i64, custom: Option<Box<dyn DecisionPolicy>>) -> (Game, Table) {
    let db = playmat_engine::cards();
    let decks = [deck("gw-company.txt"), deck("b-zombies.txt")];
    let mut game = new_duel(&db, [&decks[0], &decks[1]], seed, 0).unwrap();
    let mut table = Table::new(configs.to_vec());
    if let Some(p) = custom {
        table.set_policy(0, p);
    }
    rules::play_from_priority(&mut game, &mut table, 30);
    (game, table)
}

fn config(policy: PolicyKind) -> Config {
    Config { budget: 600, width: 4, policy, threads: 2, ..Config::default() }
}

fn ended(game: &Game) -> bool {
    game.player(0).lost || game.player(1).lost || game.turn >= 30
}

#[test]
fn every_shipped_policy_plays_a_whole_game() {
    for policy in [PolicyKind::Playouts, PolicyKind::Mcts, PolicyKind::Quick, PolicyKind::Random] {
        let (game, table) = play([Some(config(policy)), Some(config(PolicyKind::Playouts))], 3, None);
        assert!(!table.stopped_early(), "{policy:?} stopped early at turn {}", game.turn);
        assert!(ended(&game), "{policy:?}: the game ends");
        assert_eq!(table.policy_name(0).as_deref(), Some(policy.name()));
    }
}

#[test]
fn policies_parse_from_the_setting() {
    assert_eq!("MCTS".parse::<PolicyKind>(), Ok(PolicyKind::Mcts));
    assert_eq!("".parse::<PolicyKind>(), Ok(PolicyKind::Playouts));
    assert_eq!("search".parse::<PolicyKind>(), Ok(PolicyKind::Playouts));
    assert!("jev".parse::<PolicyKind>().is_err());
}

/// A policy from outside the crate: always the last candidate, counting its calls
struct Last(Arc<AtomicUsize>);

impl DecisionPolicy for Last {
    fn name(&self) -> &str {
        "last"
    }
    fn decide(&mut self, d: &Decision) -> Option<Choice> {
        self.0.fetch_add(1, Ordering::Relaxed);
        assert!(d.candidates.len() >= 2 && d.candidates[0] == *d.quick);
        Some(Choice { answer: d.candidates.last().unwrap().clone(), nodes: 0 })
    }
}

#[test]
fn a_set_policy_answers_its_seat() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (game, table) = play(
        [Some(config(PolicyKind::Playouts)), Some(config(PolicyKind::Playouts))],
        4,
        Some(Box::new(Last(calls.clone()))),
    );
    assert!(ended(&game) && !table.stopped_early());
    assert!(calls.load(Ordering::Relaxed) > 5, "asked {} times", calls.load(Ordering::Relaxed));
    assert_eq!(table.policy_name(0).as_deref(), Some("last"));
    assert_eq!(table.stats.iter().filter(|s| s.seat == 0).count(), calls.load(Ordering::Relaxed));
}

#[test]
fn mcts_replays_the_same_game_from_the_same_seed() {
    let run = || {
        let (game, table) = play([Some(config(PolicyKind::Mcts)), Some(config(PolicyKind::Playouts))], 7, None);
        (
            game.turn,
            game.player(0).life,
            game.player(1).life,
            table.answers,
            table.stats.iter().map(|s| s.nodes).sum::<u64>(),
        )
    };
    assert_eq!(run(), run());
}
