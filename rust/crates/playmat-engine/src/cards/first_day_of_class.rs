//! First Day of Class: "Whenever a creature you control enters this turn, put a +1/+1 counter on it and it gains
//! haste until end of turn. Learn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "FirstDayOfClass.FirstDayOfClassTriggeredAbility",
    "new EntersBattlefieldControlledTriggeredAbility(Zone.BATTLEFIELD, new AddCountersTargetEffect(CounterType.P1P1.createInstance()), StaticFilters.FILTER_PERMANENT_CREATURE, false, SetTargetPointer.PERMANENT)",
    creature_of_yours,
)];

/// a creature you control entering, pointed at: the base trigger is all of it
fn creature_of_yours(_: &Game, _: &StackItem) -> bool {
    true
}
