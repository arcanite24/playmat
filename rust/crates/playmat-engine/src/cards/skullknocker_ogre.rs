//! Skullknocker Ogre: "Whenever Skullknocker Ogre deals damage to an opponent, that player discards a card at random.
//! If the player does, they draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SkullknockerOgre.SkullknockerOgreEffect", apply)];

/// the damaged player (the trigger's pointer) discards a card at random; one discarded, they draw a card
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    if !discard_at_random(game, p, 1).is_empty() {
        draw(game, p, 1);
    }
    Some(())
}
