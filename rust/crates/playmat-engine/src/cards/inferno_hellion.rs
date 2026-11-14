//! Inferno Hellion: "Trample. At the beginning of each end step, if this creature attacked or blocked this turn,
//! its owner shuffles it into their library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("InfernoHellion.InfernoHellionCondition", attacked_or_blocked)];

/// `AttackedThisTurnWatcher` or `BlockedThisTurnWatcher` saw the source, as the object it is now
fn attacked_or_blocked(game: &Game, _: Seat, source: ObjId) -> bool {
    let at = (source, game.obj(source).zcc);
    game.attacked_this_turn.contains(&at) || game.watch.blocked.contains(&at)
}
