//! No One Left Behind: "This spell costs {3} less to cast if it targets a creature card with mana value 3 or less.
//! Return target creature card from your graveyard to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("NoOneLeftBehind.NoOneLeftBehindCondition", targets_cheap_creature)];

/// the spell on the stack (none while it's only being checked as playable) targets a creature card with mana value
/// 3 or less
fn targets_cheap_creature(game: &Game, _: Seat, source: ObjId) -> bool {
    let Some(me) = game.stack.iter().find(|i| i.card == source && i.ability.is_none()) else { return false };
    me.targets.iter().flatten().any(|t| match t {
        Target::Obj(o) => game.has_type(*o, CardType::Creature) && game.obj(*o).def.mana_value() <= 3,
        _ => false,
    })
}
