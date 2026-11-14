//! `RemoveFromCombatTargetEffect`: "remove target creature from combat" (Thaumatic Compass's Spires of Orazca: "Untap
//! target attacking creature an opponent controls and remove it from combat.")

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RemoveFromCombatTargetEffect", apply)];

/// Each target still on the battlefield leaves combat (`Permanent.removeFromCombat`)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.args.is_empty() {
        return Some(());
    }
    for id in cx.objs(0) {
        if game.obj(id).zone == Zone::Battlefield {
            remove_from_combat(game, id);
        }
    }
    Some(())
}
