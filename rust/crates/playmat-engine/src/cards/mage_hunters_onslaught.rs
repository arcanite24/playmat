//! Mage Hunters' Onslaught: "Destroy target creature or planeswalker. Whenever a creature blocks this turn, its
//! controller loses 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "MageHuntersOnslaught.MageHuntersOnslaughtDelayedTriggeredAbility",
    "new BlocksAllTriggeredAbility(new LoseLifeTargetEffect(1), false, StaticFilters.FILTER_PERMANENT_CREATURE, SetTargetPointer.PLAYER)",
    any_blocker,
)];

/// a creature blocking (`CREATURE_BLOCKS`), pointing at its controller: the base trigger is all of it
fn any_blocker(_: &Game, _: &StackItem) -> bool {
    true
}
