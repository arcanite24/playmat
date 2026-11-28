//! Venat, Heart of Hydaelyn // Hydaelyn, the Mothercrystal: "Blessing of Light — At the beginning of combat on your
//! turn, put a +1/+1 counter on another target creature you control. Until your next turn, it gains indestructible.
//! If that creature is legendary, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VenatHeartOfHydaelyn.HydaelynTheMothercrystalEffect", apply)];

/// The target, still on the battlefield and legendary: you draw a card
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone == Zone::Battlefield && game.obj(o).def.supertypes.iter().any(|t| t == "LEGENDARY") {
        draw(game, cx.controller(), 1);
    }
    Some(())
}
