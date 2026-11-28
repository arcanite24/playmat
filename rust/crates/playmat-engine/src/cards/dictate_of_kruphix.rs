//! Dictate of Kruphix: "At the beginning of each player's draw step, that player draws an additional card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "DictateOfKruphix.DictateOfKruphixAbility",
    "new BeginningOfDrawTriggeredAbility(TargetController.ANY, new DrawCardTargetEffect(1), false)",
    any,
)];

/// (`DRAW_STEP_PRE` of any player, its pointer at that player: the base trigger's whole check)
fn any(_: &Game, _: &StackItem) -> bool {
    true
}
