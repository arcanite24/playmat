//! Vine Gecko: "The first kicked spell you cast each turn costs {1} less to cast."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("VineGecko.VineGeckoPredicate", first_kicked)];

/// You cast no kicked spell yet this turn (its watcher's set), and this card has kicker and is a spell on the stack
/// cast kicked (`KickerAbility.getKickedCounter` of its spell ability: nothing while only checked as playable)
fn first_kicked(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    if game.watch.kicked_cast.contains(&you) {
        return false;
    }
    game.obj(id).def.abilities.iter().any(|a| matches!(a, AbilityDef::KickerAbility { .. }))
        && game.stack.iter().any(|i| i.card == id && i.ability.is_none() && i.kicked)
}
