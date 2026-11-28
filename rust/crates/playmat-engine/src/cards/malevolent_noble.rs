//! Malevolent Noble: "{2}, Sacrifice an artifact or another creature: Put a +1/+1 counter on Malevolent Noble."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("MalevolentNoble.MalevolentNoblePredicate", artifact_or_other)];

/// An artifact, or a creature other than the source
fn artifact_or_other(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let artifact = game.has_type(id, CardType::Artifact);
    if Some(id) == source {
        return artifact;
    }
    artifact || game.has_type(id, CardType::Creature)
}
