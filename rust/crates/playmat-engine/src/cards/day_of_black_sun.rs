//! Day of Black Sun: "Each creature with mana value X or less loses all abilities until end of turn. Destroy those
//! creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("DayOfBlackSun.DayOfBlackSunPredicate", mana_value_x_or_less)];

/// `getManaValue() <= GetXValue.calculate(source)`: the X of the spell resolving (or on the stack)
fn mana_value_x_or_less(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let x = game
        .resolving_item
        .as_ref()
        .filter(|i| i.card == src && i.ability.is_none())
        .or_else(|| game.stack.iter().rev().find(|i| i.card == src && i.ability.is_none()))
        .map_or(0, |i| i.x);
    game.obj(id).def.mana_value() as i32 <= x
}
