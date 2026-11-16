//! Sab-Sunen, Luxa Embodied: "Sab-Sunen can't attack or block unless it has an even number of counters on it. At the
//! beginning of your first main phase, put a +1/+1 counter on Sab-Sunen. Then if it has an odd number of counters on
//! it, draw two cards."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("SabSunenLuxaEmbodied.SabSunenLuxaEmbodiedCondition.EVEN", even),
    Handler::condition("SabSunenLuxaEmbodied.SabSunenLuxaEmbodiedCondition.ODD", odd),
];

/// `CountersSourceCount.ANY`: every counter on it
fn counters(game: &Game, source: ObjId) -> i32 {
    game.obj(source).counters.iter().map(|(_, n)| *n).sum()
}

fn even(game: &Game, _: Seat, source: ObjId) -> bool {
    counters(game, source) % 2 == 0
}

fn odd(game: &Game, _: Seat, source: ObjId) -> bool {
    counters(game, source) % 2 == 1
}
