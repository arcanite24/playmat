//! Chandra, Chill of Compliance: "+1: Surveil 1. If you put a noncreature, nonland card into your graveyard this
//! way, put that card into your hand."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ChandraChillOfCompliance.ChandraChillOfComplianceSurveilEffect", apply)
        .effects(&["new SurveilEffect(1)"])];

/// `doSurveil(1)`; the top card, if the surveil put it into the graveyard and it's neither a creature nor a land,
/// to your hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top = game.player(you).library.first().copied();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let Some(c) = top else { return Some(()) };
    if game.obj(c).zone == Zone::Graveyard
        && !game.has_type(c, CardType::Creature)
        && !game.has_type(c, CardType::Land)
    {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}
