//! Shadow of Mortality: "If your life total is less than your starting life total, this spell costs {X} less to cast,
//! where X is the difference."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("ShadowOfMortality.ShadowOfMortalityValue", value)];

/// (every game the engine plays starts at 20 life)
fn value(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    (20 - game.player(you).life).max(0)
}
