//! Unleash Fury: "Double the power of target creature until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UnleashFury.UnleashFuryEffect", apply)];

/// `BoostTargetEffect(power, 0, EndOfTurn)` with its power as the spell resolves
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let power = game.power_toughness(o).0;
    boost(game, o, power, 0, Duration::EndOfTurn);
    Some(())
}
