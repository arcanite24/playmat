//! Siren's Ruse: "Exile target creature you control, then return that card to the battlefield under its owner's
//! control. If a Pirate was exiled this way, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SirensRuse.SirensRuseEffect", apply)
    .effects(&["new ExileThenReturnTargetEffect(true, false)"])];

/// Whether it's a Pirate as this resolves; the exile and the return (`ReturnToBattlefieldUnderYourControlTargetEffect`:
/// under your control); then the draw, if it was exiled
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let pirate = game.has_subtype(o, "Pirate");
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    if pirate {
        draw(game, you, 1);
    }
    Some(())
}
