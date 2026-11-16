//! Blue Sun's Twilight: "Gain control of target creature with mana value X or less. If X is 5 or more, create a
//! token that's a copy of that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("BlueSunsTwilight.BlueSunsTwilightCondition", x_five)];

/// The X the spell was cast with (`getSourceCostsTag("X")`), read from the spell resolving
fn x_five(game: &Game, _you: Seat, source: ObjId) -> bool {
    game.resolving_item.as_ref().is_some_and(|i| i.card == source && i.x >= 5)
}
