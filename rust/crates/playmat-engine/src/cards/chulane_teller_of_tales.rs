//! Chulane, Teller of Tales: "Whenever you cast a creature spell, draw a card, then you may put a land card from your
//! hand onto the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChulaneTellerOfTales.ChulaneTellerOfTalesEffect", apply)
    .effects(&["new DrawCardSourceControllerEffect(1)", "new PutCardFromHandOntoBattlefieldEffect(StaticFilters.FILTER_CARD_LAND_A)"])];

/// The two effects in turn, with no targets
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    cx.run_on(game, d, &cx.effects[1..2], Vec::new());
    if d.stopped() {
        return None;
    }
    Some(())
}
