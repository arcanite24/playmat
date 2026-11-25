//! Tokka & Rahzar, Terrible Twos: "Whenever a player casts a spell, if the amount of mana spent to cast it was less
//! than its mana value, Tokka & Rahzar deal 3 damage to that player."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("TokkaAndRahzarTerribleTwos.TokkaAndRahzarTerribleTwosPredicate", less_mana_spent)];

/// `getUsedManaToPay().count() < getManaValue()`: the mana spent to cast the spell against its mana value on the
/// stack (its printed cost, with X as it was cast)
fn less_mana_spent(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let o = game.obj(id);
    let Some(item) = game.stack.iter().rev().find(|i| i.card == id && i.ability.is_none() && i.delayed.is_none())
    else {
        return false;
    };
    let c = o.def.printed();
    let mv = c.mana_value() as i32 + item.x * c.x as i32;
    (o.mana_spent as i32) < mv
}
