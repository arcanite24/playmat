//! Iridescent Hornbeetle: "At the beginning of your end step, create a 1/1 green Insect creature token for each
//! +1/+1 counter you've put on creatures under your control this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("IridescentHornbeetle.IridescentHornbeetleValue", counters_put)];

/// `IridescentHornbeetleWatcher.getCounterCount`: the +1/+1 counters you put this turn (its `COUNTER_ADDED` events
/// by player, on any permanent)
fn counters_put(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.watch.p1p1_put.iter().filter(|(s, _)| *s == you).map(|(_, n)| *n).sum()
}
