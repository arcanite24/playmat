//! Ash Zealot: "Whenever a player casts a spell from a graveyard, Ash Zealot deals 3 damage to that player."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "AshZealot.AshZealotTriggeredAbility",
    "new SpellCastAllTriggeredAbility(new DamageTargetEffect(3), StaticFilters.FILTER_SPELL_A, false, SetTargetPointer.PLAYER)",
    from_graveyard,
)];

/// `checkTrigger`: the spell cast (the latest spell on the stack, its caster the trigger's player pointer) was cast
/// from a graveyard (`event.getZone() == Zone.GRAVEYARD`)
fn from_graveyard(game: &Game, t: &StackItem) -> bool {
    let Some(Target::Player(p)) = t.targets.first().and_then(|ts| ts.first()).copied() else {
        return false;
    };
    let spell = game.stack.iter().rev().find(|s| s.ability.is_none() && game.obj(s.card).zone == Zone::Stack);
    spell.is_some_and(|s| s.controller == p && game.obj(s.card).cast_from == Some(Zone::Graveyard))
}
