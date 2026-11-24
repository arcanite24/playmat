//! `UndyingAbility`: "When this creature dies, if it had no +1/+1 counters on it, return it to the battlefield under
//! its owner's control with a +1/+1 counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "UndyingAbility",
    "new DiesSourceTriggeredAbility(new ReturnToBattlefieldUnderOwnerControlWithCounterTargetEffect(CounterType.P1P1.createInstance(), false, false, false), false, SetTargetPointer.CARD)",
    no_counters,
)];

/// `checkTrigger`: the permanent that died had no +1/+1 counters on it as it last was (the latest time this card left
/// the battlefield, dying)
fn no_counters(game: &Game, t: &StackItem) -> bool {
    game.watch
        .left_battlefield
        .iter()
        .rev()
        .find(|l| l.id == t.card && l.died)
        .is_none_or(|l| !l.counters.iter().any(|(name, n)| name == "+1/+1" && *n > 0))
}
