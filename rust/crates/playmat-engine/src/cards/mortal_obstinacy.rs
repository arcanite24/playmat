//! Mortal Obstinacy: "Whenever enchanted creature deals combat damage to a player, you may sacrifice Mortal
//! Obstinacy. If you do, destroy target enchantment."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "MortalObstinacy.MortalObstinacyAbility",
    "new DealsDamageToAPlayerAttachedTriggeredAbility(new DoIfCostPaid(new DestroyTargetEffect(), new SacrificeSourceCost()), \"enchanted creature\", false, false, true)",
    check,
)];

/// `DAMAGED_PLAYER`, combat damage, by the creature this is attached to: the base trigger's event
fn check(_: &Game, _: &StackItem) -> bool {
    true
}
