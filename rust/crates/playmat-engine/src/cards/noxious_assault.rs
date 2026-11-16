//! Noxious Assault: "Creatures you control get +2/+2 until end of turn. Whenever a creature blocks this turn, its
//! controller gets a poison counter."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "NoxiousAssault.NoxiousAssaultDelayedTriggerAbility",
    "new BlocksAllTriggeredAbility(new AddPoisonCounterTargetEffect(1), false, StaticFilters.FILTER_PERMANENT_CREATURE, SetTargetPointer.PLAYER)",
    any_blocker,
)];

/// a creature blocking (`CREATURE_BLOCKS`), pointing at its controller: the base trigger is all of it
fn any_blocker(_: &Game, _: &StackItem) -> bool {
    true
}
