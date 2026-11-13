//! Warden of the Inner Sky: "As long as Warden of the Inner Sky has three or more counters on it, it has flying and
//! vigilance."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("WardenOfTheInnerSky.WardenOfTheInnerSkyCondition", three_or_more_counters)];

/// The source permanent's counters of every kind, three or more in all
fn three_or_more_counters(game: &Game, _you: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    o.zone == Zone::Battlefield && o.counters.iter().map(|(_, n)| *n).sum::<i32>() >= 3
}
