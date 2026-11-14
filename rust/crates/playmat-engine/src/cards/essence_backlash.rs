//! Essence Backlash: "Counter target creature spell. Essence Backlash deals damage equal to that spell's power to
//! its controller."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EssenceBacklash.EssenceBacklashEffect", apply)];

/// the spell's controller and power are read before the counter; the damage comes whether or not it was countered
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let spell = cx.obj(0)?;
    let p = game.obj(spell).controller;
    let power = game.power_toughness(spell).0;
    cx.run(game, d, &[EffectDef::CounterTargetEffect]);
    if d.stopped() {
        return None;
    }
    if !game.player(p).lost && power > 0 {
        deal_damage(game, cx.source_id(), Target::Player(p), power, false);
    }
    Some(())
}
