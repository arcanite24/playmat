//! Executioner's Swing: "Target creature that dealt damage this turn gets -5/-5 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("ExecutionersSwing.ExecutionersSwingPredicate", dealt_damage_this_turn)];

/// `SourceDidDamageWatcher.checkSource`: this permanent (as this object) dealt damage this turn
pub(super) fn dealt_damage_this_turn(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let zcc = game.obj(id).zcc;
    game.watch.damage.iter().any(|s| s.source == id && s.source_zcc == zcc && s.amount > 0)
}
