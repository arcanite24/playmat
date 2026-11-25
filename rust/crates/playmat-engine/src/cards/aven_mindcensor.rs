//! Aven Mindcensor: "If an opponent would search a library, that player searches the top four cards of that library
//! instead."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("AvenMindcensor.AvenMindcensorEffect", Rule::SearchLimit(top_four_for_opponents))];

/// `SEARCH_LIBRARY` of a player who is an opponent of the Mindcensor's controller: amount 4
fn top_four_for_opponents(_game: &Game, cx: &StaticCx, seat: Seat, _owner: Seat) -> Option<usize> {
    (seat != cx.controller).then_some(4)
}
