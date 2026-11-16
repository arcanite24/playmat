//! Fairgrounds Trumpeter: "At the beginning of each end step, if a +1/+1 counter was put on a permanent under your
//! control this turn, put a +1/+1 counter on Fairgrounds Trumpeter."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("FairgroundsTrumpeter.FairgroundsTrumpeterCondition", p1p1_put_on_yours)];

/// `FairgroundsTrumpeterWatcher`: a +1/+1 counter was put this turn on a permanent you control (its controller as
/// last known)
fn p1p1_put_on_yours(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.counters_put.iter().any(|(_, o, _, name, n, _)| name == "+1/+1" && *n > 0 && game.obj(*o).controller == you)
}
