//! `DealtDamageAndDiedTriggeredAbility`: "Whenever a creature dealt damage by {this} this turn dies" (Sengir Vampire)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("DealtDamageAndDiedTriggeredAbility", dealt_damage_by_source)];

/// `getDealtDamageByThisTurn` of the dying creature holds the trigger's source (`DamagedByWatcher`'s record: the
/// creature as it was then, one zone change before the graveyard it's in now)
fn dealt_damage_by_source(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let zcc = game.obj(id).zcc;
    game.damaged_by_source.iter().any(|(s, o, z)| *s == src && *o == id && (*z == zcc || *z + 1 == zcc))
}
