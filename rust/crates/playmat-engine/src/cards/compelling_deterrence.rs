//! Compelling Deterrence: "Return target nonland permanent to its owner's hand. Then that player discards a card if
//! you control a Zombie."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CompellingDeterrence.CompellingDeterrenceEffect", apply)];

/// "that player" is the permanent's controller; the Zombie is counted after the bounce
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let player = game.obj(o).controller;
    game.move_to(o, Zone::Hand);
    let you = cx.controller();
    let zombie = game.battlefield.iter().any(|p| game.obj(*p).controller == you && game.has_subtype(*p, "Zombie"));
    if zombie {
        discard(game, d, player, 1);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
