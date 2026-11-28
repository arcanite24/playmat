//! Windreader Sphinx: "Whenever a creature with flying attacks, you may draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "WindreaderSphinx.WindreaderSphinxTriggeredAbility",
    "new AttacksAllTriggeredAbility(new DrawCardSourceControllerEffect(1), true, filter, SetTargetPointer.NONE, false)",
    check,
)];

/// The base trigger's filter (any creature with flying, `ATTACKER_DECLARED`) is the whole check
fn check(_game: &Game, _t: &StackItem) -> bool {
    true
}
