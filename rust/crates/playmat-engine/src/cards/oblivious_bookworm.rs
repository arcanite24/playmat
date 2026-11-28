//! Oblivious Bookworm: "At the beginning of your end step, you may draw a card. If you do, discard a card unless a
//! permanent entered the battlefield face down under your control this turn or you turned a permanent face up this
//! turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("ObliviousBookworm.ObliviousBookwormCondition", discards)];

/// no permanent entered face down under your control this turn and you turned none face up (its watcher's set)
fn discards(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).face_down_turn != Some(game.turn)
}
