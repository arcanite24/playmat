//! Rampaging Cyclops: "This creature gets -2/-0 as long as two or more creatures are blocking it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("RampagingCyclops.RampagingCyclopsCondition", double_blocked)];

/// attacking, with two or more blockers in its combat group
fn double_blocked(game: &Game, _: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    o.zone == Zone::Battlefield && o.attacking.is_some() && live_blockers(game, source).len() > 1
}
