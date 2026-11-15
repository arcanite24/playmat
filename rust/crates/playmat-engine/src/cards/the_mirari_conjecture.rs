//! The Mirari Conjecture: "I — Return target instant card from your graveyard to your hand. II — Return target
//! sorcery card from your graveyard to your hand. III — Until end of turn, whenever you cast an instant or sorcery
//! spell, copy it. You may choose new targets for the copy."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "TheMirariConjecture.TheMirariConjectureDelayedTriggeredAbility",
    "new SpellCastControllerTriggeredAbility(new CopyTargetStackObjectEffect(true), StaticFilters.FILTER_SPELL_AN_INSTANT_OR_SORCERY, false, SetTargetPointer.SPELL)",
    any_spell,
)];

/// an instant or sorcery spell you cast (the trigger's pointer): the base trigger is all of it
fn any_spell(_: &Game, _: &StackItem) -> bool {
    true
}
