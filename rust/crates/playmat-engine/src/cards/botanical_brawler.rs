//! Botanical Brawler: "Trample. Botanical Brawler enters the battlefield with two +1/+1 counters on it. Whenever one
//! or more +1/+1 counters are put on another permanent you control, if it's the first time +1/+1 counters have been
//! put on that permanent this turn, put a +1/+1 counter on Botanical Brawler."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "BotanicalBrawler.BotanicalBrawlerTriggeredAbility",
    "new CountersAddedAllTriggeredAbility(new AddCountersSourceEffect(CounterType.P1P1.createInstance()), CounterType.P1P1, StaticFilters.FILTER_CONTROLLED_PERMANENT, TargetController.ANY, true)",
    check,
)];

/// `COUNTERS_ADDED` of +1/+1 counters on a permanent you control, the first this turn on that permanent
/// (`BoostCountersAddedFirstTimeWatcher`: the base trigger), other than Botanical Brawler itself (the pointer)
fn check(_: &Game, t: &StackItem) -> bool {
    matches!(t.targets.first().and_then(|g| g.first()), Some(Target::Obj(o)) if *o != t.card)
}
