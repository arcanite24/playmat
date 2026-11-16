//! Out of Air: "This spell costs {2} less to cast if it targets a creature spell. Counter target spell."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("OutOfAir.OutOfAirCondition", targets_creature_spell)];

/// the spell on the stack (none while it's only being checked as playable) targets a creature spell
fn targets_creature_spell(game: &Game, _: Seat, source: ObjId) -> bool {
    let Some(me) = game.stack.iter().find(|i| i.card == source && i.ability.is_none()) else { return false };
    me.targets.iter().flatten().any(|t| match t {
        Target::Obj(o) => {
            game.stack.iter().any(|i| i.card == *o && i.ability.is_none()) && game.has_type(*o, CardType::Creature)
        }
        _ => false,
    })
}
