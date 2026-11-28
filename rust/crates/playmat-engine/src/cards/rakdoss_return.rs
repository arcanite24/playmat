//! Rakdos's Return: "Rakdos's Return deals X damage to target opponent or planeswalker. That player or that
//! planeswalker's controller discards X cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RakdossReturn.RakdossReturnEffect", apply)];

/// `game.getPlayerOrPlaneswalkerController(firstTarget)` discards X
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = match cx.target(0)? {
        Target::Player(p) => p,
        Target::Obj(o) => game.obj(o).controller,
    };
    discard(game, d, p, cx.item.x);
    if d.stopped() {
        return None;
    }
    Some(())
}
