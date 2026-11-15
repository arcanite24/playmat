//! `ProtectedByOpponentPredicate`: a battle one of your opponents protects ("each battle they protect": Joyful
//! Stormsculptor)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("ProtectedByOpponentPredicate", protected_by_opponent)];

fn protected_by_opponent(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.obj(id).protector.is_some_and(|p| p != you)
}
