//! Assassin's Strike: "Destroy target creature. Its controller discards a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AssassinsStrike.AssassinsStrikeEffect", apply)];

/// After the destruction: the creature's controller (its last known one: the engine keeps the controller of a
/// creature that stayed, and the owner of one that went to the graveyard) discards a card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let p = game.obj(o).controller;
    discard(game, d, p, 1);
    if d.stopped() {
        return None;
    }
    Some(())
}
