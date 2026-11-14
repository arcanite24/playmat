//! Stocking the Pantry: "Whenever you put one or more +1/+1 counters on a creature you control, put a supply counter
//! on this enchantment."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "StockingThePantry.StockingThePantryTriggeredAbility",
    "new PutCounterOnPermanentTriggeredAbility(new AddCountersSourceEffect(CounterType.SUPPLY.createInstance()), CounterType.P1P1, StaticFilters.FILTER_CONTROLLED_CREATURE)",
    any,
)];

/// (the base trigger's event, player and filter are the whole check)
fn any(_: &Game, _: &StackItem) -> bool {
    true
}
