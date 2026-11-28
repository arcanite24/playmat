//! Aggressive Mining: "You can't play lands. Sacrifice a land: Draw two cards. Activate only once each turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("AggressiveMining.AggressiveMiningEffect", Rule::CantPlayLands(you_cant))];

/// `PLAY_LAND` by the source's controller
fn you_cant(_: &Game, cx: &StaticCx, seat: Seat) -> bool {
    seat == cx.controller
}
