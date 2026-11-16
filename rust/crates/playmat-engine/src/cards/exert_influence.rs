//! Exert Influence: "Converge — Gain control of target creature if its power is less than or equal to the number of
//! colors of mana spent to cast this spell."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ExertInfluence.ExertInfluenceEffect", apply)
    .effects(&["new GainControlTargetEffect(Duration.Custom, true)"])];

/// `ColorsOfManaSpentToCastCount` against the target's power, then control of it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let creature = cx.obj(0)?;
    let colors = game.obj(cx.source_id()).spent_colors.iter().filter(|n| **n > 0).count() as i32;
    if game.power_toughness(creature).0 <= colors {
        cx.run(game, d, &cx.effects[..1]);
    }
    (!d.stopped()).then_some(())
}
