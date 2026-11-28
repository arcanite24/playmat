//! Drogskol Reaver: "Whenever you gain life, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "DrogskolReaver.DrogskolReaverAbility",
    "new GainLifeControllerTriggeredAbility(new DrawCardSourceControllerEffect(1))",
    check,
)];

/// `GAINED_LIFE` by you: the base trigger's own event, nothing more
fn check(_game: &Game, _t: &StackItem) -> bool {
    true
}
