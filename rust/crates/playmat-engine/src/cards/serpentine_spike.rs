//! Serpentine Spike: "Devoid. Serpentine Spike deals 2 damage to target creature, 3 damage to another target
//! creature, and 4 damage to a third target creature. If a creature dealt damage this way would die this turn,
//! exile it instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SerpentineSpike.SerpentineSpikeEffect", apply)];

/// Each target group in order, the ones still on the battlefield
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for (i, n) in [2, 3, 4].into_iter().enumerate() {
        if let Some(o) = cx.obj(i) {
            if game.obj(o).zone == Zone::Battlefield {
                deal_damage(game, cx.source_id(), Target::Obj(o), n, false);
            }
        }
    }
    Some(())
}
