//! Hollowmurk Siege: "As this enchantment enters, choose Sultai or Abzan. • Sultai — Whenever a counter is put on a
//! creature you control, draw a card. This ability triggers only once each turn. • Abzan — Whenever you attack, put a
//! +1/+1 counter on target attacking creature. It gains menace until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "HollowmurkSiege.HollowmurkSiegeSultaiTriggeredAbility",
    "new CounterAddedAllTriggeredAbility(new DrawCardSourceControllerEffect(1), null, StaticFilters.FILTER_CONTROLLED_CREATURE, TargetController.ANY).setTriggersLimitEachTurn(1)",
    check,
)];

/// `COUNTER_ADDED` of any counter, put by anyone, on a creature you control (the base trigger)
fn check(_: &Game, _: &StackItem) -> bool {
    true
}
