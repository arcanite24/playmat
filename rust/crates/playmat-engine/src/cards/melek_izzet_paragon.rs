//! Melek, Izzet Paragon: "Whenever you cast an instant or sorcery spell from your library, copy it. You may choose
//! new targets for the copy."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "MelekIzzetParagon.MelekIzzetParagonTriggeredAbility",
    "new SpellCastControllerTriggeredAbility(new CopyTargetStackObjectEffect(), StaticFilters.FILTER_SPELL_AN_INSTANT_OR_SORCERY, false, SetTargetPointer.SPELL)",
    from_library,
)];

/// `checkTrigger`: the spell was cast from a library (`event.getZone() == Zone.LIBRARY`), one you own
fn from_library(game: &Game, t: &StackItem) -> bool {
    let Some(spell) = t.cast_spell.or(match t.targets.first().and_then(|ts| ts.first()) {
        Some(Target::Obj(o)) => Some(*o),
        _ => None,
    }) else {
        return false;
    };
    game.obj(spell).cast_from == Some(Zone::Library) && game.obj(spell).owner == t.controller
}
