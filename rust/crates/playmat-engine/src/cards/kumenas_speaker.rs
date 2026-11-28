//! Kumena's Speaker: "Kumena's Speaker gets +1/+1 as long as you control another Merfolk or an Island."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("KumenasSpeaker.KumenasSpeakerPredicate", merfolk_or_island)];

/// An Island, or a Merfolk other than the source
fn merfolk_or_island(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    game.has_subtype(id, "Island") || (source != Some(id) && game.has_subtype(id, "Merfolk"))
}
