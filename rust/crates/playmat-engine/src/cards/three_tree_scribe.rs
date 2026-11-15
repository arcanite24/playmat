//! Three Tree Scribe: "Whenever Three Tree Scribe or another creature you control leaves the battlefield without
//! dying, put a +1/+1 counter on target creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ThreeTreeScribe.ThreeTreeScribeTriggeredAbility",
    "new LeavesBattlefieldAllTriggeredAbility(new AddCountersTargetEffect(CounterType.P1P1.createInstance()), StaticFilters.FILTER_CONTROLLED_CREATURE)",
    without_dying,
)];

/// it went anywhere but a graveyard
fn without_dying(game: &Game, item: &StackItem) -> bool {
    item.left.is_some_and(|o| game.obj(o).zone != Zone::Graveyard)
}
