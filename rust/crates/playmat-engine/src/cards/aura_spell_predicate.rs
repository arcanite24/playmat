//! `AuraSpellPredicate`: "an Aura spell" (Brine Comber)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("AuraSpellPredicate", aura_spell)];

/// a spell on the stack whose characteristics have the Aura subtype
fn aura_spell(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.obj(id).zone == Zone::Stack && game.has_subtype(id, "Aura")
}
