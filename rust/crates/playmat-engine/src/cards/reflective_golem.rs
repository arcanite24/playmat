//! Reflective Golem: "Whenever you cast an instant or sorcery spell that targets only Reflective Golem, you may pay
//! {2}. If you do, copy that spell. You may choose new targets for the copy."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ReflectiveGolem.ReflectiveGolemTriggeredAbility",
    "new SpellCastControllerTriggeredAbility(new DoIfCostPaid(new CopyTargetStackObjectEffect(), new GenericManaCost(2)), StaticFilters.FILTER_SPELL_AN_INSTANT_OR_SORCERY, false, SetTargetPointer.SPELL)",
    targets_it,
)];

/// `checkTrigger`: Reflective Golem is among the spell's targets (Java counts its distinct targets equal to the
/// source: once is enough, other targets don't stop it)
fn targets_it(game: &Game, t: &StackItem) -> bool {
    let Some(spell) = t.cast_spell.or(match t.targets.first().and_then(|ts| ts.first()) {
        Some(Target::Obj(o)) => Some(*o),
        _ => None,
    }) else {
        return false;
    };
    let me = t.card;
    game.stack
        .iter()
        .find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none())
        .is_some_and(|i| i.targets.iter().flatten().any(|x| *x == Target::Obj(me)))
}
