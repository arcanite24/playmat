//! Blossombind: "Enchant creature. When this Aura enters, tap enchanted creature. Enchanted creature can't become
//! untapped and can't have counters put on it."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::rule("Blossombind.BlossombindUntapEffect", Rule::CantUntap(enchanted)),
    Handler::rule("Blossombind.BlossombindCounterEffect", Rule::CantHaveCounters(enchanted_counters)),
];

/// `UNTAP` of the permanent the Aura is attached to, replaced with nothing
fn enchanted(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.attached(cx.source) == Some(id)
}

/// `CAN_ADD_COUNTERS` of the enchanted permanent, any counter
fn enchanted_counters(game: &Game, cx: &StaticCx, id: ObjId, _counter: &str) -> bool {
    game.attached(cx.source) == Some(id)
}
