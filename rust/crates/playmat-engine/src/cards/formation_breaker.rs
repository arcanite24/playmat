//! Formation Breaker: "Creatures with power less than this creature's power can't block it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("FormationBreaker.FormationBreakerPredicate", weaker)];

/// its power is less than the source's, while the source is on the battlefield
fn weaker(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(s) = source.filter(|s| game.obj(*s).zone == Zone::Battlefield) else { return false };
    game.power_toughness(id).0 < game.power_toughness(s).0
}
