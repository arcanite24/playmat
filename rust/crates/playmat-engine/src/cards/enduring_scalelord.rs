//! Enduring Scalelord: "Flying. Whenever one or more +1/+1 counters are put on another creature you control, you
//! may put a +1/+1 counter on Enduring Scalelord."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "EnduringScalelord.EnduringScalelordTriggeredAbility",
    "new PutCounterOnPermanentTriggeredAbility(new AddCountersSourceEffect(CounterType.P1P1.createInstance()), CounterType.P1P1, StaticFilters.FILTER_ANOTHER_CREATURE_YOU_CONTROL, false, true)",
    counters_on_another,
)];

/// `COUNTERS_ADDED` of +1/+1 counters on another creature you control: all of it the base's event and filter
fn counters_on_another(_: &Game, _: &StackItem) -> bool {
    true
}
