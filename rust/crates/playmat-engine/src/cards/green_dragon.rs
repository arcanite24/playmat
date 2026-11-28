//! Green Dragon: "Flying. Poison Breath — When Green Dragon enters, until end of turn, whenever a creature an opponent
//! controls is dealt damage, destroy it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "GreenDragon.GreenDragonDelayedTriggeredAbility",
    "new DealtDamageAnyTriggeredAbility(new DestroyTargetEffect(), StaticFilters.FILTER_OPPONENTS_PERMANENT_CREATURE, SetTargetPointer.PERMANENT, false)",
    opponents_creature,
)];

/// `DAMAGED_PERMANENT` on a creature an opponent controls, the pointer at it: the base trigger is all of it
fn opponents_creature(_: &Game, _: &StackItem) -> bool {
    true
}
