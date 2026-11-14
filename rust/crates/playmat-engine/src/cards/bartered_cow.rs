//! Bartered Cow: "When Bartered Cow dies or when you discard it, create a Food token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("BarteredCow.BarteredCowPredicate", itself)];

/// the card discarded is the source itself
fn itself(_game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    source == Some(id)
}
