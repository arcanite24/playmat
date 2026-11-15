//! `AttackedThisTurnPredicate` (framework): "creatures that attacked this turn", a permanent that
//! `AttackedThisTurnWatcher` saw attack, as the same object (`MageObjectReference.refersTo`).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("AttackedThisTurnPredicate", attacked)];

fn attacked(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.attacked_this_turn.contains(&(id, game.obj(id).zcc))
}
