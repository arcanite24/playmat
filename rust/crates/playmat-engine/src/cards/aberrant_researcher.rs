//! Aberrant Researcher: "At the beginning of your upkeep, mill a card. If an instant or sorcery card was milled this
//! way, transform Aberrant Researcher."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AberrantResearcher.AberrantResearcherEffect", apply)
    .effects(&["new MillCardsControllerEffect(1)", "new TransformSourceEffect()"])];

/// the library's top card, if the mill put it into the graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let top = game.player(cx.controller()).library.first().copied();
    cx.run(game, d, &cx.effects[..1]);
    let Some(c) = top else { return Some(()) };
    let spell = game.has_type(c, CardType::Instant) || game.has_type(c, CardType::Sorcery);
    if game.obj(c).zone == Zone::Graveyard && spell {
        cx.run(game, d, &cx.effects[1..2]);
    }
    Some(())
}
