//! Unstoppable Slasher: "When Unstoppable Slasher dies, if it had no counters on it, return it to the battlefield
//! tapped under its owner's control with two stun counters on it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("UnstoppableSlasher.UnstoppableSlasherCondition", had_no_counters)];

/// The source (`getSourcePermanentOrLKI`) had no counters on it: its own on the battlefield, else its last known ones
fn had_no_counters(game: &Game, _you: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    let counters = if o.zone == Zone::Battlefield { &o.counters } else { &o.lki_counters };
    counters.iter().map(|(_, n)| *n).sum::<i32>() == 0
}
