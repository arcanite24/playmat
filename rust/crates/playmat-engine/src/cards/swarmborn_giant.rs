//! Swarmborn Giant: "When you're dealt combat damage, sacrifice Swarmborn Giant."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SwarmbornGiant.SwarmbornGiantTriggeredAbility",
    "new YouDealtCombatDamageTriggeredAbility(new SacrificeSourceEffect())",
    any_batch,
)];
