//! Tunnel Tipster: "At the beginning of your end step, if a face-down creature entered the battlefield under your
//! control this turn, put a +1/+1 counter on Tunnel Tipster."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("TunnelTipster.TunnelTipsterCondition", entered)];

/// its watcher's set: the controllers of face-down creatures that entered this turn
fn entered(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.watch.face_down_creature_entered.contains(&you)
}
