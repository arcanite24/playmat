//! Coiling Stalker: "Ninjutsu {1}{G}. Whenever Coiling Stalker deals combat damage to a player, put a +1/+1 counter
//! on target creature you control that doesn't have a +1/+1 counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("CoilingStalker.CoilingStalkerPredicate", no_p1p1)];

/// it has no +1/+1 counter
fn no_p1p1(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.obj(id).counter("+1/+1") == 0
}
