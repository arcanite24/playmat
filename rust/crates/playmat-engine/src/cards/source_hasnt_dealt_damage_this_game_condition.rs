//! `SourceHasntDealtDamageThisGameCondition`: "if it hasn't dealt damage yet" (Karakyk Guardian, Palladia-Mors):
//! `DealtDamageThisGameWatcher` has no damage from the source as the permanent it is

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("SourceHasntDealtDamageThisGameCondition", hasnt_dealt_damage)];

/// the source as it is now hasn't dealt damage since it entered (the engine marks a permanent that deals damage)
fn hasnt_dealt_damage(game: &Game, _: Seat, source: ObjId) -> bool {
    !game.obj(source).marks.iter().any(|m| m == "dealt damage")
}
