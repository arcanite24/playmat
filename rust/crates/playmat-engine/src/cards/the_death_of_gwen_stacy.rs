//! The Death of Gwen Stacy: "I — Destroy target creature. II — Each player may discard a card. Each player who doesn't
//! loses 3 life. III — Exile any number of target players' graveyards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheDeathOfGwenStacy.TheDeathOfGwenStacyEffect", apply)];

/// Chapter II: each player in turn order from you chooses up to one card (`TargetDiscard(0, 1)`); then, in the same
/// order, each discards the card chosen or loses 3 life
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let players: Vec<Seat> =
        (0..game.seats()).map(|i| (you + i) % game.seats()).filter(|p| !game.player(*p).lost).collect();
    let mut chosen = Vec::new();
    for &p in &players {
        let hand = game.player(p).hand.clone();
        let c = if hand.is_empty() { Vec::new() } else { choose_cards(game, d, p, &hand, &hand, 0, 1, "discard")? };
        chosen.push(c);
    }
    for (p, cards) in players.into_iter().zip(chosen) {
        if cards.is_empty() {
            lose_life(game, p, 3);
            continue;
        }
        for c in cards {
            if game.obj(c).zone == Zone::Hand {
                discard_card(game, p, c);
                game.discarded_cards();
            }
        }
    }
    Some(())
}
