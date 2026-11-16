//! The Sibsig Ceremony: "Whenever a creature you control enters, if you cast it, destroy that creature, then create a
//! 2/2 black Zombie Druid creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("TheSibsigCeremony.TheSibsigCeremonyPredicate", was_cast)];

/// the permanent was the spell just before (`getSpellOrLKIStack` with one zone change less): it resolved as a spell
fn was_cast(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let o = game.obj(id);
    o.zone == Zone::Battlefield && o.cast
}
