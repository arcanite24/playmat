//! Urgoros, the Empty One: "Whenever Urgoros, the Empty One deals combat damage to a player, that player discards a
//! card at random. If the player can't, you draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UrgorosTheEmptyOne.UrgorosTheEmptyOneEffect", apply)];

/// with an empty hand, you draw; else the damaged player (the trigger's pointer) discards one at random
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    if game.player(p).hand.is_empty() {
        draw(game, cx.controller(), 1);
    } else {
        discard_at_random(game, p, 1);
    }
    Some(())
}
