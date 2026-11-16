//! Death Frenzy: "All creatures get -2/-2 until end of turn. Whenever a creature dies this turn, you gain 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "DeathFrenzy.DeathFrenzyDelayedTriggeredAbility",
    "new DiesCreatureTriggeredAbility(new GainLifeEffect(1), false, StaticFilters.FILTER_PERMANENT_CREATURE)",
    any_creature,
)];

/// a creature dying (`FILTER_PERMANENT_CREATURES`): the base trigger is all of it
fn any_creature(_: &Game, _: &StackItem) -> bool {
    true
}
