//! `ExaltedAbility`: "Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ExaltedAbility",
    "new AttacksAloneControlledTriggeredAbility(new BoostTargetEffect(1, 1, Duration.EndOfTurn), true, false)",
    any,
)];

/// (the base `AttacksAloneControlledTriggeredAbility`, its pointer at the attacker: its whole check)
fn any(_: &Game, _: &StackItem) -> bool {
    true
}
