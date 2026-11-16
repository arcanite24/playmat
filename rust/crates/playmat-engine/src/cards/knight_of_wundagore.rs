//! Knight of Wundagore: "Trample. Whenever you put a +1/+1 counter on another creature, put a +1/+1 counter on this
//! creature. This ability triggers only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "KnightOfWundagore.KnightOfWundagoreTriggeredAbility",
    "new CounterAddedAllTriggeredAbility(new AddCountersSourceEffect(CounterType.P1P1.createInstance()), CounterType.P1P1, StaticFilters.FILTER_ANOTHER_CREATURE, TargetController.YOU)",
    check,
)];

/// `COUNTER_ADDED` of a +1/+1 counter you put on another creature (the base trigger)
fn check(_: &Game, _: &StackItem) -> bool {
    true
}
