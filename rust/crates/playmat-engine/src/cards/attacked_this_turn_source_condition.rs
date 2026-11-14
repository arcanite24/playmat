//! `AttackedThisTurnSourceCondition` (The Lunar Whale): "as long as this attacked this turn".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("AttackedThisTurnSourceCondition", attacked)];

/// `AttackedThisTurnWatcher.checkIfAttacked(source permanent)`: this object attacked this turn
fn attacked(game: &Game, _you: Seat, source: ObjId) -> bool {
    let zcc = game.obj(source).zcc;
    game.attacked_this_turn.iter().any(|(o, z)| *o == source && *z == zcc)
}
