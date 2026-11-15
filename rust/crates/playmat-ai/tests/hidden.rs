//! Samples of what a seat can't see (`decision::Root::sample`, `Config::hide`)

use playmat_ai::decision::{Hidden, Root};
use playmat_ai::duel::{new_duel, Deck};
use playmat_engine::state::Zone;

fn decks() -> (Deck, Deck) {
    let a = Deck::parse("a", "20 Mountain\n20 Lightning Strike");
    let b = Deck::parse("b", "20 Forest\n10 Llanowar Elves\n10 Gnarlback Rhino");
    (a, b)
}

#[test]
fn a_sample_redeals_the_opponents_hand_and_keeps_the_seats_own() {
    let db = playmat_engine::cards();
    let (a, b) = decks();
    let game = new_duel(&db, [&a, &b], 7, 0).unwrap();
    let mut hidden = Hidden::new(0);
    hidden.observe(&game);
    let root = Root { snapshot: game.clone(), since: Vec::new(), hidden: Some(hidden), playout: 1, rollout: None };
    let mut changed = 0;
    for seed in 0..20 {
        let s = root.sample(seed);
        // seat 0's hand is its own: never changed
        assert_eq!(s.player(0).hand, game.player(0).hand);
        // the opponent still has seven cards and the same cards over hand and library
        assert_eq!(s.player(1).hand.len(), 7);
        let mut before: Vec<_> = game.player(1).hand.iter().chain(game.player(1).library.iter()).copied().collect();
        let mut after: Vec<_> = s.player(1).hand.iter().chain(s.player(1).library.iter()).copied().collect();
        before.sort();
        after.sort();
        assert_eq!(before, after);
        for id in &s.player(1).hand {
            assert_eq!(s.obj(*id).zone, Zone::Hand);
        }
        for id in &s.player(1).library {
            assert_eq!(s.obj(*id).zone, Zone::Library);
        }
        if s.player(1).hand != game.player(1).hand {
            changed += 1;
        }
        // seat 0's library order is unknown to it too
        assert_ne!(s.player(0).library, game.player(0).library);
    }
    assert!(changed >= 19, "the opponent's hand was redealt in {changed} of 20 samples");
    // the same seed gives the same sample
    assert_eq!(root.sample(3).player(1).hand, root.sample(3).player(1).hand);
}

#[test]
fn a_card_the_seat_saw_stays_in_the_opponents_hand() {
    let db = playmat_engine::cards();
    let (a, b) = decks();
    let mut game = new_duel(&db, [&a, &b], 7, 0).unwrap();
    // a card of the opponent's on the battlefield, seen, then back in its hand (bounced)
    let card = game.player(1).library[0];
    game.player_mut(1).library.remove(0);
    game.move_to(card, Zone::Battlefield);
    let mut hidden = Hidden::new(0);
    hidden.observe(&game);
    game.move_to(card, Zone::Hand);
    if !game.player(1).hand.contains(&card) {
        game.player_mut(1).hand.push(card);
    }
    let root = Root { snapshot: game.clone(), since: Vec::new(), hidden: Some(hidden), playout: 1, rollout: None };
    for seed in 0..10 {
        assert!(root.sample(seed).player(1).hand.contains(&card));
    }
}
