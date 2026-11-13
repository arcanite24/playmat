//! `DiscardHandAllEffect`: "each player discards their hand" (Dragon Mage, Wheel of Fortune and the like)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DiscardHandAllEffect", apply)];

/// Each player from you in turn order discards their whole hand (`player.discard(hand)`: no choice, the hand's
/// order), each player's discards one batch
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        for c in game.player(p).hand.clone() {
            discard_card(game, p, c);
        }
        game.discarded_cards();
    }
    Some(())
}
