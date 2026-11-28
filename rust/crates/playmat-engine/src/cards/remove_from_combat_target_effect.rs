//! `RemoveFromCombatTargetEffect`: "remove it from combat" (Spires of Orazca, the back of Thaumatic Compass)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RemoveFromCombatTargetEffect", apply)];

/// each target still on the battlefield stops attacking or blocking (`PermanentImpl.removeFromCombat`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for o in cx.objs(0) {
        if game.obj(o).zone == Zone::Battlefield {
            remove_from_combat(game, o);
        }
    }
    Some(())
}
