//! Ingenious Leonin: "{3}{W}: Put a +1/+1 counter on another target attacking creature you control. If that creature
//! is a Cat, it gains first strike until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("IngeniousLeonin.IngeniousLeoninEffect", apply)
    .effects(&["new GainAbilityTargetEffect(FirstStrikeAbility.getInstance(), Duration.EndOfTurn)"])];

/// First strike until end of turn when it's a Cat
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield || !game.has_subtype(o, "Cat") {
        return None;
    }
    cx.run(game, d, &cx.effects[..1]);
    Some(())
}
