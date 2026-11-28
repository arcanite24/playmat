//! Kellan, Inquisitive Prodigy: "Whenever Kellan attacks, destroy up to one target artifact. If you controlled that
//! permanent, draw a card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("KellanInquisitiveProdigy.KellanInquisitiveProdigyEffect", apply)];

/// No artifact chosen (or it's gone) does nothing; else whether you controlled it is read before it's destroyed, and
/// you draw then whether or not it was destroyed
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let mine = game.obj(o).controller == cx.controller();
    destroy(game, o);
    if mine {
        draw(game, cx.controller(), 1);
    }
    Some(())
}
