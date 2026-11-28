//! `BlockedThisTurnPredicate`: a creature that blocked this turn (`BlockedThisTurnWatcher`, Sizzling Barrage)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("BlockedThisTurnPredicate", blocked_this_turn)];

/// declared as a blocker this turn, as the object it is now
fn blocked_this_turn(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.watch.blocked.contains(&(id, game.obj(id).zcc))
}
