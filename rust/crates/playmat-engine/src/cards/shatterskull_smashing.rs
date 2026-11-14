//! Shatterskull Smashing: "If X is 6 or more, Shatterskull Smashing deals twice X damage divided as you choose among
//! them instead."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("ShatterskullSmashing.ShatterskullSmashingCondition", x_six_or_more),
    Handler::value("ShatterskullSmashing.ShatterskullSmashingAdjuster", divided),
];

/// The spell's X (its costs tag) is 6 or more: the resolving spell's, or the one on the stack
fn x_six_or_more(game: &Game, _you: Seat, source: ObjId) -> bool {
    game.resolving_item
        .as_ref()
        .filter(|i| i.card == source && i.ability.is_none())
        .or_else(|| game.stack.iter().rev().find(|i| i.card == source && i.ability.is_none()))
        .is_some_and(|i| i.x >= 6)
}

/// `ShatterskullSmashingAdjuster`'s divided amount: X, or twice X if X is 6 or more
fn divided(_game: &Game, _you: Seat, _source: ObjId, x: i32) -> i32 {
    if x >= 6 {
        2 * x
    } else {
        x
    }
}
