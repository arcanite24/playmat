//! Valeron Wardens: "Renown 2. Whenever a creature you control becomes renowned, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ValeronWardens.ValeronWardensTriggeredAbility",
    "new BecomesRenownedAllTriggeredAbility(new DrawCardSourceControllerEffect(1), StaticFilters.FILTER_CONTROLLED_PERMANENT)",
    check,
)];

/// `BECOMES_RENOWNED` of a permanent you control (the base trigger)
fn check(_: &Game, _: &StackItem) -> bool {
    true
}
