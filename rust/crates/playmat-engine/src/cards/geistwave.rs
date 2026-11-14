//! Geistwave: "Return target nonland permanent to its owner's hand. If you controlled that permanent, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Geistwave.GeistwaveEffect", apply)];

/// Whether you controlled it is read before it moves
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let you = cx.controller();
    let yours = game.obj(o).controller == you;
    game.move_to(o, Zone::Hand);
    if yours {
        draw(game, you, 1);
    }
    Some(())
}
