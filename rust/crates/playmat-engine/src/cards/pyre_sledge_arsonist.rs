//! Pyre-Sledge Arsonist: "{1}, {T}: This creature deals X damage to any target, where X is the number of
//! permanents you've sacrificed this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("PyreSledgeArsonist.PyreSledgeArsonistValue", you_sacrificed)];

/// `PyreSledgeArsonistWatcher`: `SACRIFICED_PERMANENT` events of yours this turn
fn you_sacrificed(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.watch.sacrificed.iter().filter(|(s, _)| *s == you).count() as i32
}
