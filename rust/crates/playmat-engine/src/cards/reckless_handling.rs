//! Reckless Handling: "Search your library for an artifact card, reveal it, put it into your hand, shuffle, then
//! discard a card at random. If an artifact card was discarded this way, Reckless Handling deals 2 damage to each
//! opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RecklessHandling.RecklessHandlingEffect", apply)];

/// a card discarded at random; an artifact card, then 2 damage to each opponent
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(&card) = discard_at_random(game, you, 1).first() else { return Some(()) };
    if !game.has_type(card, CardType::Artifact) {
        return Some(());
    }
    let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
    for p in opponents {
        deal_damage(game, cx.source_id(), Target::Player(p), 2, false);
    }
    Some(())
}
