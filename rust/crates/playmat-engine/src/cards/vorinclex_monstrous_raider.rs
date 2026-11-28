//! Vorinclex, Monstrous Raider: "Trample, haste. If you would put one or more counters on a permanent or player, put
//! twice that many of each of those kinds of counters on that permanent or player instead. If an opponent would put
//! one or more counters on a permanent or player, they put half that many of each of those kinds of counters on
//! that permanent or player instead, rounded down."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "VorinclexMonstrousRaider.VorinclexMonstrousRaiderEffect",
    &[EventType::AddCounters],
    applies,
    replace,
)];

/// counters put on a permanent (the engine's `ADD_COUNTERS` is a permanent's) by you or an opponent
fn applies(_game: &Game, _cx: &StaticCx, ev: &Event) -> bool {
    ev.target.is_some()
}

/// twice as many by you (`CardUtil.overflowMultiply`), half as many (rounded down) by an opponent
fn replace(_game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if ev.player == cx.controller {
        ev.amount = ev.amount.saturating_mul(2);
    } else {
        ev.amount = ev.amount.div_euclid(2);
    }
    false
}
