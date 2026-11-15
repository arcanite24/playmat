//! Stalwart Successor: "Menace. Whenever one or more counters are put on a creature you control, if it's the first
//! time counters have been put on that creature this turn, put a +1/+1 counter on that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "StalwartSuccessor.StalwartSuccessorTriggeredAbility",
    "new CountersAddedAllTriggeredAbility(new AddCountersTargetEffect(CounterType.P1P1.createInstance()), null, StaticFilters.FILTER_CONTROLLED_CREATURE, TargetController.ANY, true)",
    check,
)];

/// `COUNTERS_ADDED` on a creature you control, the first this turn on that creature (`CountersAddedFirstTimeWatcher`,
/// checked as it triggers: the base trigger), the pointer at it
fn check(_: &Game, _: &StackItem) -> bool {
    true
}
