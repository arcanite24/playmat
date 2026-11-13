//! Blot Out the Sky: "If X is 6 or more, destroy all noncreature, nonland permanents."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("BlotOutTheSky.BlotOutTheSkyCondition", holds)];

/// The X the spell was cast with (`getSourceCostsTag("X")`), read from the spell resolving
fn holds(game: &Game, _you: Seat, source: ObjId) -> bool {
    game.resolving_item.as_ref().is_some_and(|i| i.card == source && i.x >= 6)
}
