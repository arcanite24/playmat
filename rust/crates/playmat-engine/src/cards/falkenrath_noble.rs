//! Falkenrath Noble: "Whenever Falkenrath Noble or another creature dies, target player loses 1 life and you gain 1
//! life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "FalkenrathNoble.FalkenrathNobleTriggeredAbility",
    "new DiesThisOrAnotherTriggeredAbility(new LoseLifeTargetEffect(1), false)",
    any,
)];

/// (a dies event of this or of a creature, as it last was on the battlefield: the base trigger's whole check; the
/// constructor adds the gain and the target player)
fn any(_: &Game, _: &StackItem) -> bool {
    true
}
