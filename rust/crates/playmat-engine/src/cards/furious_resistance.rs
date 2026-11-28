//! Furious Resistance: "Target blocking creature gets +3/+0 and gains first strike until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FuriousResistance.FuriousResistanceEffect", apply).effects(&[
    "new BoostTargetEffect(3, 0, Duration.EndOfTurn)",
    "new GainAbilityTargetEffect(FirstStrikeAbility.getInstance(), Duration.EndOfTurn)",
])];

/// The boost, then first strike, both on the target
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    cx.run(game, d, cx.effects);
    Some(())
}
