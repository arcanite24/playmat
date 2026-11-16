//! Jolly Gerbils: "Whenever you give a gift, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "JollyGerbils.JollyGerbilsTriggeredAbility",
    "new GaveGiftTriggeredAbility(new DrawCardSourceControllerEffect(1))",
    any,
)];

/// `GAVE_GIFT` of a gift you gave (the base trigger is all of it)
fn any(_: &Game, _: &StackItem) -> bool {
    true
}
