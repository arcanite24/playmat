//! Enchanted River's Grasp: "Enchant creature. When this Aura enters, tap enchanted creature and remove all counters
//! from it. Enchanted creature loses all abilities and doesn't untap during its controller's untap step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EnchantedRiversGrasp.EnchantedRiversGraspEffect", apply)];

/// The permanent the Aura that entered is attached to loses every counter on it
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = game.attached(cx.source_id())?;
    let counters = game.obj(o).counters.clone();
    for (name, n) in counters {
        if n > 0 {
            game.obj_mut(o).add_counters(&name, -n);
        }
    }
    game.obj_mut(o).counters.retain(|(_, k)| *k > 0);
    Some(())
}
