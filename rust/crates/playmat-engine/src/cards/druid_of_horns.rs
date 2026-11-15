//! Druid of Horns: "Whenever you cast an Aura spell that targets Druid of Horns, create a 3/3 green Beast creature
//! token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "DruidOfHorns.DruidOfHornsTriggeredAbility",
    "new SpellCastControllerTriggeredAbility(new CreateTokenEffect(new BeastToken()), StaticFilters.FILTER_SPELL_A, false)",
    check,
)];

/// `SPELL_CAST` by you (the base trigger) of an Aura spell one of whose targets is this. Java checks it only as it
/// triggers: as it resolves it holds
fn check(game: &Game, t: &StackItem) -> bool {
    if game.resolving_item.as_ref().is_some_and(|r| r.card == t.card && r.ability == t.ability) {
        return true;
    }
    let Some(spell) = t.cast_spell else { return false };
    game.has_subtype(spell, "Aura")
        && game.stack.iter().any(|i| {
            i.card == spell && i.ability.is_none() && i.targets.iter().flatten().any(|x| *x == Target::Obj(t.card))
        })
}
