//! Emeria Shepherd: "Landfall — Whenever a land you control enters, you may return target nonland permanent card
//! from your graveyard to your hand. If that land is a Plains, you may return that nonland permanent card to the
//! battlefield instead."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("EmeriaShepherd.EmeriaShepherdReturnToHandTargetEffect", apply).effects(&[
        "new ReturnFromGraveyardToHandTargetEffect()",
        "new ReturnFromGraveyardToBattlefieldTargetEffect()",
    ])];

/// The land that entered (`getTriggeringPermanent`) is a Plains: asked whether the card goes to the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(land) = cx.item.entering else { return Some(()) };
    let to_battlefield =
        game.has_subtype(land, "Plains") && choose_use(game, d, cx.controller(), "Put the card to battlefield instead?")?;
    cx.run(game, d, &cx.effects[usize::from(to_battlefield)..][..1]);
    Some(())
}
