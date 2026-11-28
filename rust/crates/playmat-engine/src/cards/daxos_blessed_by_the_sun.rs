//! Daxos, Blessed by the Sun: "Whenever another creature you control enters or dies, you gain 1 life." (its
//! toughness: `SetBaseToughnessSourceEffect`)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "DaxosBlessedByTheSun.DaxosBlessedByTheSunAbility",
    "new OrTriggeredAbility(Zone.BATTLEFIELD, new GainLifeEffect(1), new EntersBattlefieldControlledTriggeredAbility(null, StaticFilters.FILTER_CONTROLLED_ANOTHER_CREATURE), new DiesCreatureTriggeredAbility(null, false, StaticFilters.FILTER_ANOTHER_CREATURE_YOU_CONTROL))",
    any,
)];

fn any(_: &Game, _: &StackItem) -> bool {
    true
}
