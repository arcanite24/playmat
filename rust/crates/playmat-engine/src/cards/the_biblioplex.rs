//! The Biblioplex: "Activate only if you have exactly zero or seven cards in hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("TheBiblioplex.TheBiblioplexCondition", zero_or_seven)];

fn zero_or_seven(game: &Game, you: Seat, _source: ObjId) -> bool {
    matches!(game.player(you).hand.len(), 0 | 7)
}
