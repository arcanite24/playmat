//! Finale of Devastation: "If X is 10 or more, creatures you control get +X/+X and gain haste until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FinaleOfDevastation.FinaleOfDevastationEffect", apply).effects(&[
    "new BoostControlledEffect(GetXValue.instance, GetXValue.instance, Duration.EndOfTurn)",
    "new GainAbilityControlledEffect(HasteAbility.getInstance(), Duration.EndOfTurn, new FilterCreaturePermanent())",
])];

/// X (`getSourceCostsTag("X")`) of 10 or more: both continuous effects, as Java adds them
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.item.x >= 10 {
        cx.run(game, d, cx.effects);
    }
    Some(())
}
