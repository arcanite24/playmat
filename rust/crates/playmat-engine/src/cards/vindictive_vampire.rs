//! Vindictive Vampire: "Whenever another creature you control dies, Vindictive Vampire deals 1 damage to each
//! opponent and you gain 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "VindictiveVampire.VindictiveVampireTriggeredAbility",
    "new DiesCreatureTriggeredAbility($1, false, StaticFilters.FILTER_CONTROLLED_ANOTHER_CREATURE) /* zone: $0 */",
    any_batch,
)];
