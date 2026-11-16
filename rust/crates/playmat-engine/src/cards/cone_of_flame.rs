//! Cone of Flame: "Cone of Flame deals 1 damage to any target, 2 damage to another target, and 3 damage to a third
//! target."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ConeOfFlame.ConeOfFlameEffect", apply)];

/// Each target in order, the first still legal of its group: 1 damage, then 2, then 3
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for i in 0..cx.legal.len() {
        if let Some(t) = cx.target(i) {
            deal_damage(game, cx.source_id(), t, i as i32 + 1, false);
        }
    }
    Some(())
}
