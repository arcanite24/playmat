//! Stensia Masquerade: "Attacking creatures you control have first strike. Whenever a Vampire you control deals
//! combat damage to a player, put a +1/+1 counter on it. Madness {2}{R}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "StensiaMasquerade.StensiaMasqueradeTriggeredAbility",
    "new DealsDamageToAPlayerAllTriggeredAbility(new AddCountersTargetEffect(CounterType.P1P1.createInstance()), new FilterControlledPermanent(SubType.VAMPIRE), false, SetTargetPointer.PERMANENT, true)",
    always,
)];

/// `DAMAGED_PLAYER`, combat damage from a Vampire you control (pointed at it): the base trigger says all of it
fn always(_: &Game, _: &StackItem) -> bool {
    true
}
