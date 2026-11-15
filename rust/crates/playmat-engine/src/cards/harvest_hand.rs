//! Harvest Hand: "When Harvest Hand dies, return it to the battlefield transformed under your control." Scrounged
//! Scythe: "Equipped creature gets +1/+1. As long as equipped creature is a Human, it has menace. Equip {2}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HarvestHand.HarvestHandReturnTransformedEffect", apply)];

/// as Ojer Taq's, untapped (`moveCards` with the card's back face up)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    super::ojer_taq_deepest_foundation::return_transformed(game, d, cx, false)
}
