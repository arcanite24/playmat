//! `OtherThanEnteringPredicate`: "other than the entering permanent" (Roiling Canopy's "if you control at least five
//! other Forests")

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("OtherThanEnteringPredicate", other_than_entering)];

/// Not the permanent whose entering the trigger saw (its `permanentEnteringBattlefield` value), as the trigger is
/// checked or resolves; true with no such trigger
fn other_than_entering(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let item = game.checking_item.as_ref().or(game.resolving_item.as_ref());
    item.and_then(|t| t.entering) != Some(id)
}
