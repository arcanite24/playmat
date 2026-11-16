//! The search plan's pieces (#390): the fitted score and mulligans by simulation

use playmat_ai::duel::{new_duel, Deck};
use playmat_ai::fitted;
use playmat_ai::mulligan;
use playmat_ai::search::Config;
use playmat_engine::card::CardType;
use playmat_engine::state::{Game, ObjId, Zone};

fn game() -> Game {
    let db = playmat_engine::cards();
    let a = Deck::parse("a", "24 Mountain\n36 Lightning Strike");
    let b = Deck::parse("b", "20 Forest\n10 Llanowar Elves\n10 Gnarlback Rhino");
    new_duel(&db, [&a, &b], 11, 0).unwrap()
}

/// Seat 0's hand set to `lands` Mountains and the rest Lightning Strikes, swapped with its library
fn set_hand(g: &mut Game, lands: usize) {
    let all: Vec<ObjId> = g.player(0).hand.iter().chain(g.player(0).library.iter()).copied().collect();
    let (l, s): (Vec<ObjId>, Vec<ObjId>) = all.iter().partition(|id| g.obj(**id).is(CardType::Land));
    let hand: Vec<ObjId> = l[..lands].iter().chain(s[..7 - lands].iter()).copied().collect();
    let library: Vec<ObjId> = all.iter().copied().filter(|id| !hand.contains(id)).collect();
    for id in &hand {
        g.obj_mut(*id).zone = Zone::Hand;
    }
    for id in &library {
        g.obj_mut(*id).zone = Zone::Library;
    }
    g.player_mut(0).hand = hand;
    g.player_mut(0).library = library;
}

#[test]
fn the_fitted_score_is_antisymmetric() {
    let g = game();
    let (f0, f1) = (fitted::features(&g, 0), fitted::features(&g, 1));
    for (a, b) in f0.iter().zip(&f1) {
        assert!((a + b).abs() < 1e-9, "{a} against {b}");
    }
    let w = fitted::weights();
    assert!((fitted::logit(&g, 0, w) + fitted::logit(&g, 1, w)).abs() < 1e-9);
}

#[test]
fn the_simulation_mulligans_no_landers_and_keeps_good_hands() {
    let cfg = Config { mulligan_sim: true, ..Config::default() };
    let mut g = game();
    set_hand(&mut g, 0);
    assert!(mulligan::wants(&g, 0, 0, Some(&cfg)), "no lands");
    set_hand(&mut g, 7);
    assert!(mulligan::wants(&g, 0, 0, Some(&cfg)), "seven lands");
    set_hand(&mut g, 3);
    assert!(!mulligan::wants(&g, 0, 0, Some(&cfg)), "three lands");
    // on six, a two-lander is kept
    set_hand(&mut g, 2);
    assert!(!mulligan::wants(&g, 0, 1, Some(&cfg)), "two lands after a mulligan");
}

#[test]
fn the_simulation_bottoms_the_extra_land_of_a_flood() {
    let cfg = Config { mulligan_sim: true, ..Config::default() };
    let mut g = game();
    set_hand(&mut g, 6);
    let card = mulligan::bottom(&g, 0, Some(&cfg));
    assert!(g.obj(card).is(CardType::Land));
}
