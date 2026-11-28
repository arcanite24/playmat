//! `FilterCreatureAttackingYou`: "creature that's attacking you", a creature whose defender is the filter's player
//! itself (`Combat.getDefenderId`: attacking a planeswalker they control doesn't count). The transpiler adds this
//! after the filter's creature and attacking predicates.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("FilterCreatureAttackingYou", attacking_you)];

fn attacking_you(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let o = game.obj(id);
    o.attacking == Some(you) && o.attacking_pw.is_none()
}
