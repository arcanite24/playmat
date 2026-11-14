//! Revelation of Power: "Target creature gets +2/+2 until end of turn. If it has a counter on it, it also gains
//! flying and lifelink until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RevelationOfPower.RevelationOfPowerEffect", apply).effects(&[
    "new GainAbilityTargetEffect(FlyingAbility.getInstance(), Duration.EndOfTurn)",
    "new GainAbilityTargetEffect(LifelinkAbility.getInstance(), Duration.EndOfTurn)",
])];

/// any kind of counter at all
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield || !game.obj(o).counters.iter().any(|(_, n)| *n > 0) {
        return Some(());
    }
    cx.run(game, d, &cx.effects[..2]);
    (!d.stopped()).then_some(())
}
