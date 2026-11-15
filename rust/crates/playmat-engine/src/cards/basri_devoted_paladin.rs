//! Basri, Devoted Paladin: "+1: Put a +1/+1 counter on up to one target creature. It gains vigilance until end of
//! turn. −1: Whenever a creature attacks this turn, put a +1/+1 counter on it. −6: Creatures you control get +2/+2 and
//! gain flying until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "BasriDevotedPaladin.BasriDevotedPaladinTriggeredAbility",
    "new AttacksAllTriggeredAbility(new AddCountersTargetEffect(CounterType.P1P1.createInstance()), false, StaticFilters.FILTER_PERMANENT_CREATURE, SetTargetPointer.PERMANENT, false)",
    any_attacker,
)];

/// a creature attacking, the counter's target: the base trigger is all of it
fn any_attacker(_: &Game, _: &StackItem) -> bool {
    true
}
