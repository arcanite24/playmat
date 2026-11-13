//! `RemoveAllCountersPermanentTargetEffect` (Hapatra's Mark): "Remove all -1/-1 counters from it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("RemoveAllCountersPermanentTargetEffect", apply).args(&["counter_type"])];

/// Every counter on the first target, of the `CounterType` argument's kind when there is one, is removed
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let only = cx.args.first();
    let counters = game.obj(o).counters.clone();
    for (name, n) in counters {
        if only.is_none_or(|k| *k == name) && n > 0 {
            game.obj_mut(o).add_counters(&name, -n);
        }
    }
    Some(())
}
