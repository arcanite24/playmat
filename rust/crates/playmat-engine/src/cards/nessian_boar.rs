//! Nessian Boar: "All creatures able to block Nessian Boar do so. Whenever Nessian Boar becomes blocked by a creature,
//! that creature's controller draws a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NessianBoar.NessianBoarEffect", apply)];

/// the blocking creature (the trigger's target pointer), if it's still on the battlefield
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    draw(game, game.obj(o).controller, 1);
    Some(())
}
