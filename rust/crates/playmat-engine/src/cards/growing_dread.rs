//! Growing Dread: "Flash. When Growing Dread enters, manifest dread. Whenever you turn a permanent face up, put a
//! +1/+1 counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "GrowingDread.GrowingDreadTriggeredAbility",
    "new TurnedFaceUpAllTriggeredAbility(new AddCountersTargetEffect(CounterType.P1P1.createInstance()), StaticFilters.FILTER_PERMANENT, true)",
    you_turned_it_up,
)];

/// you turned it face up (the event's player): the player who controls the permanent turned face up (checked
/// while it's on the battlefield; gone, the counter has nothing to go on anyway)
fn you_turned_it_up(game: &Game, item: &StackItem) -> bool {
    match item.targets.first().and_then(|ts| ts.first()) {
        Some(Target::Obj(o)) => game.obj(*o).zone != Zone::Battlefield || game.obj(*o).controller == item.controller,
        _ => false,
    }
}
