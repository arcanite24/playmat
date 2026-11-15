//! Dovin, Grand Arbiter: "+1: Until end of turn, whenever a creature you control deals combat damage to a player,
//! put a loyalty counter on Dovin, Grand Arbiter."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "DovinGrandArbiter.DovinGrandArbiterDelayedTriggeredAbility",
    "new DealsDamageToAPlayerAllTriggeredAbility(new AddCountersSourceEffect(CounterType.LOYALTY.createInstance()), StaticFilters.FILTER_CONTROLLED_CREATURE, false, SetTargetPointer.NONE, true)",
    yours_dealt_combat_damage,
)];

/// a creature you control dealing combat damage to a player: the base trigger is all of it
fn yours_dealt_combat_damage(_: &Game, _: &StackItem) -> bool {
    true
}
