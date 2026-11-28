//! Bred for the Hunt: "Whenever a creature you control with a +1/+1 counter on it deals combat damage to a player, you
//! may draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "BredForTheHunt.BredForTheHuntTriggeredAbility",
    "new DealsDamageToAPlayerAllTriggeredAbility(new DrawCardSourceControllerEffect(1), StaticFilters.FILTER_CONTROLLED_CREATURE, true, SetTargetPointer.PERMANENT, true)",
    check,
)];

/// the creature that dealt the damage (the pointer) has a +1/+1 counter
fn check(game: &Game, t: &StackItem) -> bool {
    match t.targets.first().and_then(|g| g.first()) {
        Some(Target::Obj(o)) => game.obj(*o).counter("+1/+1") > 0,
        _ => false,
    }
}
