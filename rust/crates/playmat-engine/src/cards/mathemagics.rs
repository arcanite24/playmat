//! Mathemagics: "Target player draws 2ˣ cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("Mathemagics.MathemagicsValue", value)];

/// `CardUtil.overflowExp(2, X)`: capped at `Integer.MAX_VALUE`
fn value(_game: &Game, _you: Seat, _source: ObjId, x: i32) -> i32 {
    if x < 0 {
        return 0;
    }
    2i32.checked_pow(x as u32).unwrap_or(i32::MAX)
}
