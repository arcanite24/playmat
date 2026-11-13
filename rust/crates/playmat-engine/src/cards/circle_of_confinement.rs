//! Circle of Confinement: "Whenever an opponent casts a Vampire spell with the same name as a card exiled with
//! Circle of Confinement, you gain 2 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "CircleOfConfinement.CircleOfConfinementTriggeredAbility",
    "new SpellCastOpponentTriggeredAbility(Zone.BATTLEFIELD, new GainLifeEffect(2), StaticFilters.FILTER_SPELL_A, false, SetTargetPointer.SPELL)",
    same_name_vampire,
)];

/// `checkTrigger`: the spell (the trigger's pointer) is a Vampire with the same name as a card in this Circle's
/// exile zone (`ExileUntilSourceLeavesEffect`'s, for the Circle as it is now)
fn same_name_vampire(game: &Game, t: &StackItem) -> bool {
    let Some(Target::Obj(spell)) = t.targets.first().and_then(|ts| ts.first()).copied() else {
        return false;
    };
    if !game.has_subtype(spell, "Vampire") {
        return false;
    }
    let src = game.obj(t.card);
    let name = &game.obj(spell).def.name;
    game.exiled_until_leaves
        .iter()
        .any(|(s, z, o)| *s == t.card && *z == src.zcc && game.obj(*o).zone == Zone::Exile && &game.obj(*o).def.name == name)
}
