//! Promising Duskmage: "When Promising Duskmage dies, if it had a +1/+1 counter on it, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("PromisingDuskmage.PromisingDuskmageCondition", had_counter)];

/// its counters as it last was on the battlefield
fn had_counter(game: &Game, _you: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    let counters = if o.zone == Zone::Battlefield { &o.counters } else { &o.lki_counters };
    counters.iter().any(|(k, n)| k == "+1/+1" && *n > 0)
}
