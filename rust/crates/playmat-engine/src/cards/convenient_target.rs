//! Convenient Target: "Enchant creature. When this Aura enters, suspect enchanted creature. Enchanted creature gets
//! +1/+1. {2}{R}: Return this card from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ConvenientTarget.ConvenientTargetEffect", apply)];

/// the creature the Aura is (or, gone, was last) attached to, while it's on the battlefield
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = game.attached(cx.source_id()) else { return Some(()) };
    if game.obj(o).zone == Zone::Battlefield {
        game.obj_mut(o).set_suspected(true);
    }
    Some(())
}
