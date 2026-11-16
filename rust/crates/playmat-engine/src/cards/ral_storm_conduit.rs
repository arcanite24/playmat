//! Ral, Storm Conduit: "Whenever you cast or copy an instant or sorcery spell, Ral, Storm Conduit deals 1 damage to
//! target opponent or planeswalker."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "RalStormConduit.RalStormConduitTriggeredAbility",
    "new MagecraftAbility(new DamageTargetEffect(1))",
    any_batch,
)];
