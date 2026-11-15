//! Warhost's Frenzy: "Kicker {B}. Creatures you control get +2/+0 until end of turn. If this spell was kicked,
//! whenever a creature you control dies this turn, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "WarhostsFrenzy.WarhostsFrenzyDelayedTriggeredAbility",
    "new DiesCreatureTriggeredAbility(new DrawCardSourceControllerEffect(1), false, StaticFilters.FILTER_CONTROLLED_CREATURE)",
    any_creature_you_control,
)];

/// a creature you control dying (the base trigger is all of it)
fn any_creature_you_control(_: &Game, _: &StackItem) -> bool {
    true
}
