//! Growing Dread: "Flash. When Growing Dread enters, manifest dread. Whenever you turn a permanent face up, put a
//! +1/+1 counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "GrowingDread.GrowingDreadTriggeredAbility",
    "new TurnedFaceUpAllTriggeredAbility(new AddCountersTargetEffect(CounterType.P1P1.createInstance()), StaticFilters.FILTER_PERMANENT, true)",
    you_turned_it_up,
)];

/// you turned it face up (the event's player, as `turn_face_up_by` records it; Hauntwoods Shrieker turns up a
/// permanent it doesn't control)
fn you_turned_it_up(game: &Game, item: &StackItem) -> bool {
    match item.targets.first().and_then(|ts| ts.first()) {
        Some(Target::Obj(o)) => match game.watch.turned_face_up_by.iter().rev().find(|(id, _)| id == o) {
            Some((_, by)) => *by == item.controller,
            None => game.obj(*o).zone != Zone::Battlefield || game.obj(*o).controller == item.controller,
        },
        _ => false,
    }
}
