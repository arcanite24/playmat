//! `AttachedToAttachedPredicate`: an Aura or Equipment attached to the permanent the filter's source is attached to
//! (Thran Power Suit: "+1/+1 for each Aura and Equipment attached to it").

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("AttachedToAttachedPredicate", attached_to_same)];

/// `input.getObject().getAttachedTo()` is the source's `getAttachedTo()` (both attached to something)
fn attached_to_same(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let Some(to) = game.attached(src) else { return false };
    game.attached(id) == Some(to)
}
