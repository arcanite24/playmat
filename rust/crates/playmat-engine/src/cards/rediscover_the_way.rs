//! Rediscover the Way: "I, II — Look at the top three cards of your library. Put one of them into your hand and the
//! rest on the bottom of your library in any order. III — Whenever you cast a noncreature spell this turn, target
//! creature you control gains double strike until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "RediscoverTheWay.RediscoverTheWayTriggeredAbility",
    "new SpellCastControllerTriggeredAbility(new GainAbilityTargetEffect(DoubleStrikeAbility.getInstance()), StaticFilters.FILTER_SPELL_A_NON_CREATURE, false)",
    any_spell,
)];

/// a noncreature spell you cast (`SPELL_CAST`): the base trigger is all of it
fn any_spell(_: &Game, _: &StackItem) -> bool {
    true
}
