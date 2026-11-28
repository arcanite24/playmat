//! Axgard Artisan: "Whenever one or more +1/+1 counters are put on Axgard Artisan for the first time each turn,
//! create a Treasure token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "AxgardArtisan.AxgardArtisanTriggeredAbility",
    "new CountersAddedAllTriggeredAbility(new CreateTokenEffect(new TreasureToken()), CounterType.P1P1, StaticFilters.FILTER_PERMANENT, TargetController.ANY, true)",
    check,
)];

/// `COUNTERS_ADDED` of +1/+1 counters, the first this turn (`BoostCountersAddedFirstTimeWatcher`: the base
/// trigger), on Axgard Artisan itself (the pointer)
fn check(_: &Game, t: &StackItem) -> bool {
    matches!(t.targets.first().and_then(|g| g.first()), Some(Target::Obj(o)) if *o == t.card)
}
