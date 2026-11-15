//! Avacyn's Judgment: "Madness {X}{R}. Avacyn's Judgment deals 2 damage divided as you choose among any number of
//! targets. If this spell's madness cost was paid, it deals X damage divided as you choose among those creatures
//! and/or players instead."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("AvacynsJudgment.AvacynsJudgmentManacostVariableValue", amount)];

/// `getManaCostsToPay().getVariableCosts()`: a spell cast for a cost with {X} (its madness cost: madness casts the
/// card with that cost as its own) deals X, otherwise 2
fn amount(game: &Game, _you: Seat, source: ObjId, x: i32) -> i32 {
    if game.obj(source).def.cost.x > 0 {
        x
    } else {
        2
    }
}
