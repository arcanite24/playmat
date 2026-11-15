//! Treacherous Greed: "As an additional cost to cast this spell, sacrifice a creature that dealt damage this turn.
//! Draw three cards. Each opponent loses 3 life and you gain 3 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("TreacherousGreed.TreacherousGreedPredicate", dealt_damage)];

/// `TreacherousGreedWatcher`: the permanent, as the object it is now, was the source of damage dealt this turn
fn dealt_damage(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let zcc = game.obj(id).zcc;
    game.watch.damage.iter().any(|d| d.source == id && d.source_zcc == zcc)
}
