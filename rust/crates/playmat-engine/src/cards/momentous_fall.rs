//! Momentous Fall: "As an additional cost to cast this spell, sacrifice a creature. You draw cards equal to the
//! sacrificed creature's power, then you gain life equal to its toughness."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MomentousFall.MomentousFallEffect", apply).effects(&[
    "new DrawCardSourceControllerEffect(SacrificeCostCreaturesPower.instance)",
    "new GainLifeEffect(SacrificeCostCreaturesToughness.instance)",
])];

/// The sacrificed creature's power as it last was: that many cards, if any; then its toughness in life, if any
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &cx.effects[..1]);
    cx.run(game, d, &cx.effects[1..]);
    Some(())
}
