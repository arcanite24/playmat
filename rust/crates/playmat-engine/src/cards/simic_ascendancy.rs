//! Simic Ascendancy: "Whenever one or more +1/+1 counters are put on a creature you control, put that many growth
//! counters on Simic Ascendancy."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SimicAscendancy.SimicAscendancyTriggeredAbility",
    "new PutCounterOnPermanentTriggeredAbility(new AddCountersSourceEffect(CounterType.GROWTH.createInstance(), new EffectKeyValue(\"countersAdded\")), CounterType.P1P1, StaticFilters.FILTER_CONTROLLED_CREATURE, true, false)",
    not_itself,
)];

/// `COUNTERS_ADDED` of +1/+1 counters on a creature you control (the base's filter), not on the Ascendancy itself
fn not_itself(game: &Game, t: &StackItem) -> bool {
    let _ = game;
    !matches!(t.targets.first().and_then(|g| g.first()), Some(Target::Obj(o)) if *o == t.card)
}
