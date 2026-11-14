//! Moonsilver Key: "{1}, {T}, Sacrifice Moonsilver Key: Search your library for an artifact card with a mana
//! ability or a basic land card, reveal it, put it into your hand, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("MoonsilverKey.MoonsilverKeyPredicate", mana_artifact_or_basic)];

/// a basic land card, or an artifact card with a mana ability (`ManaAbility.class::isInstance`)
fn mana_artifact_or_basic(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let def = &game.obj(id).def;
    if game.has_type(id, CardType::Land) && def.supertypes.iter().any(|s| s == "BASIC") {
        return true;
    }
    game.has_type(id, CardType::Artifact) && def.abilities.iter().any(|a| a.mana().is_some())
}
