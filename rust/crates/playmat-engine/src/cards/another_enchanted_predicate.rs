//! `AnotherEnchantedPredicate`: "another …", not the permanent the source (an Aura) is attached to (Due Diligence)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("AnotherEnchantedPredicate", another_enchanted)];

/// `getSourcePermanentOrLKI(game).getAttachedTo()` isn't `id`
fn another_enchanted(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return true };
    let s = game.obj(src);
    let to = if s.zone == Zone::Battlefield { s.attached_to } else { s.lki_attached_to };
    to.map(|(o, _)| o) != Some(id)
}
