//! Wildwood Scourge: "Wildwood Scourge enters with X +1/+1 counters on it. Whenever one or more +1/+1 counters are
//! put on another non-Hydra creature you control, put a +1/+1 counter on Wildwood Scourge."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "WildwoodScourge.WildwoodScourgeTriggeredAbility",
    "new PutCounterOnPermanentTriggeredAbility(new AddCountersSourceEffect(CounterType.P1P1.createInstance()), CounterType.P1P1, StaticFilters.FILTER_ANOTHER_CREATURE_YOU_CONTROL, true, false)",
    non_hydra,
)];

/// `COUNTERS_ADDED` of +1/+1 counters on another creature you control (the base's filter, its pointer at that
/// permanent) that isn't a Hydra. Java checks it only as the counters are put: resolving, it still resolves. (Java
/// doesn't ask who put the counters; the base wants them put by you, which is who puts them on your creatures.)
fn non_hydra(game: &Game, item: &StackItem) -> bool {
    if game.resolving_item.as_ref().is_some_and(|r| r.card == item.card && r.ability == item.ability) {
        return true;
    }
    match item.targets.first().and_then(|g| g.first()) {
        Some(Target::Obj(o)) => !game.has_subtype(*o, "Hydra"),
        _ => false,
    }
}
