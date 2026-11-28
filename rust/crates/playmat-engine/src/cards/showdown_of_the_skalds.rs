//! Showdown of the Skalds: "I — Exile the top four cards of your library. Until the end of your next turn, you may
//! play those cards. II, III — Whenever you cast a spell this turn, put a +1/+1 counter on target creature you
//! control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ShowdownOfTheSkalds.ShowdownOfTheSkaldsDelayedTriggeredAbility",
    "new SpellCastControllerTriggeredAbility(new AddCountersTargetEffect(CounterType.P1P1.createInstance()), false)",
    any_spell,
)];

/// any spell you cast (`SPELL_CAST`): the base trigger is all of it
fn any_spell(_: &Game, _: &StackItem) -> bool {
    true
}
