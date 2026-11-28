//! Consuming Ashes: "Exile target creature. If it had mana value 3 or less, surveil 2."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ConsumingAshes.ConsumingAshesEffect", apply)
    .effects(&["new ExileTargetEffect()", "new SurveilEffect(2)"])];

/// `permanent.moveToExile`, then the surveil reads the exiled permanent's mana value
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let mv = game.obj(o).def.mana_value();
    cx.run(game, d, &cx.effects[..1]);
    if mv <= 3 {
        cx.run_on(game, d, &cx.effects[1..2], Vec::new());
    }
    if d.stopped() {
        return None;
    }
    Some(())
}
