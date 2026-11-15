//! Cloaked Cadet: "Training. Whenever one or more +1/+1 counters are put on one or more Humans you control, draw a
//! card. This ability triggers only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "CloakedCadet.CloakedCadetTriggeredAbility",
    "new CounterAddedAllTriggeredAbility(new DrawCardSourceControllerEffect(1), CounterType.P1P1, StaticFilters.FILTER_CONTROLLED_PERMANENT, TargetController.ANY).setTriggersLimitEachTurn(1)",
    human,
)];

/// `COUNTER_ADDED` of a +1/+1 counter on a permanent you control (the base trigger), a Human
fn human(game: &Game, t: &StackItem) -> bool {
    matches!(t.targets.first().and_then(|g| g.first()), Some(Target::Obj(o)) if game.has_subtype(*o, "Human"))
}
