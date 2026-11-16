//! Acquisition Octopus: "Whenever this Equipment or equipped creature deals combat damage to a player, draw a card.
//! Reconfigure {2}"

use super::*;

/// `DAMAGED_PLAYER` by combat damage from this or the creature it's attached to: Thassa's Emissary's trigger
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "AcquisitionOctopus.AcquisitionOctopusTriggeredAbility",
    "new ThassasEmissaryTriggeredAbility(new DrawCardSourceControllerEffect(1), false)",
    any_batch,
)];
