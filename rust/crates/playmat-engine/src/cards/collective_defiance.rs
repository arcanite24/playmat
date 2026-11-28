//! Collective Defiance: "Escalate {1} … • Target player discards all the cards in their hand, then draws that many
//! cards. • …"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CollectiveDefiance.CollectiveDefianceEffect", apply)];

/// `targetPlayer.discard(hand)` (no choice, one batch), then as many cards drawn as were discarded
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let hand = game.player(p).hand.clone();
    for c in &hand {
        discard_card(game, p, *c);
    }
    game.discarded_cards();
    draw(game, p, hand.len());
    Some(())
}
