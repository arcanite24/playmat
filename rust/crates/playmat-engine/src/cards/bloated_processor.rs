//! Bloated Processor: "When this creature dies, incubate X, where X is its power."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("BloatedProcessor.BloatedProcessorEffect", apply).tokens(&["IncubatorToken"])];

/// `IncubateEffect.doIncubate` with the power it last had (`getSourcePermanentOrLKI`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let power = cx.item.lki_power.unwrap_or_else(|| game.power_toughness(cx.source_id()).0);
    let e = EffectDef::IncubateEffect { amount: power, token: cx.tokens[0].clone() };
    cx.run(game, d, &[e]);
    (!d.stopped()).then_some(())
}
