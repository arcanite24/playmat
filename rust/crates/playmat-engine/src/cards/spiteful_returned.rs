//! Spiteful Returned: "Bestow {3}{B}. Whenever Spiteful Returned or enchanted creature attacks, defending player
//! loses 2 life. Enchanted creature gets +1/+1."

use super::*;

/// (`checkTrigger`: a creature, when this attacks; an Aura, when the creature it's attached to does; the defending
/// player its pointer)
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SpitefulReturned.SpitefulReturnedTriggeredAbility",
    "new AttacksThisOrAttachedTriggeredAbility($0)",
    any_batch,
)];
