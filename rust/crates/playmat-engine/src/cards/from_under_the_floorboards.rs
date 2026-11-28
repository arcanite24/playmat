//! From Under the Floorboards: "Madness {X}{B}{B}. Create three tapped 2/2 black Zombie creature tokens and you gain 3
//! life. If this spell's madness cost was paid, instead create X of those tokens and you gain X life."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("FromUnderTheFloorboards.FromUnderTheFloorboardsManacostVariableValue", amount)];

/// `getManaCostsToPay().getVariableCosts()`: only the madness cost has an {X}, so with it the spell's X, else 3. The
/// spell cast for its madness cost is on the stack with that cost as its mana cost.
fn amount(game: &Game, _: Seat, source: ObjId, x: i32) -> i32 {
    if game.obj(source).def.cost.x > 0 {
        x
    } else {
        3
    }
}
