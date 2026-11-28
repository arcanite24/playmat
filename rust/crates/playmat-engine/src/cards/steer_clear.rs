//! Steer Clear: "Steer Clear deals 2 damage to target attacking or blocking creature. Steer Clear deals 4 damage to
//! that creature instead if you controlled a Mount as you cast this spell."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SteerClear.SteerClearEffect", apply)
    .effects(&["new DamageTargetEffect(2)", "new DamageTargetEffect(4)"])];

/// 4 when its watcher saw this spell cast while its caster controlled a Mount, else 2
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mount = game.watch.cast_with_mount.contains(&cx.source);
    let e = &cx.effects[if mount { 1 } else { 0 }];
    cx.run(game, d, std::slice::from_ref(e));
    if d.stopped() {
        return None;
    }
    Some(())
}
