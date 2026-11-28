//! Sarkhan's Whelp: "Whenever you activate an ability of a Sarkhan planeswalker, Sarkhan's Whelp deals 1 damage to
//! any target."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SarkhansWhelp.SarkhansWhelpTriggeredAbility",
    "new ActivatePlaneswalkerLoyaltyAbilityTriggeredAbility(new DamageTargetEffect(1), SubType.SARKHAN, SetTargetPointer.NONE)",
    check,
)];

/// `ACTIVATED_ABILITY` by you, its source a Sarkhan planeswalker (a planeswalker's activated abilities are its
/// loyalty abilities: the base trigger's event)
fn check(_: &Game, _: &StackItem) -> bool {
    true
}
