//! Thallid Omnivore: "{1}, Sacrifice another creature: Thallid Omnivore gets +2/+2 until end of turn. If a Saproling
//! was sacrificed this way, you gain 2 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ThallidOmnivore.ThallidOmnivoreEffect", apply)];

/// The creature the cost sacrificed (`SacrificeTargetCost.getPermanents`), a Saproling as it was sacrificed
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.item.sacrificed.first().is_some_and(|o| game.obj(*o).def.subtypes.iter().any(|s| s == "Saproling")) {
        gain_life(game, cx.controller(), 2);
    }
    Some(())
}
