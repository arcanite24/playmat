//! Avenging Arrow: "Destroy target creature that dealt damage this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("AvengingArrow.AvengingArrowPredicate", dealt_damage)];

/// `SourceDidDamageWatcher.checkSource`: the permanent, as the object it is now, dealt damage this turn
fn dealt_damage(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let zcc = game.obj(id).zcc;
    game.watch.damage.iter().any(|d| d.source == id && d.source_zcc == zcc)
}
