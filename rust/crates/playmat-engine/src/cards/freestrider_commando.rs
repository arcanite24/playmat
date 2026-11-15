//! Freestrider Commando: "This creature enters with two +1/+1 counters on it if it wasn't cast or no mana was spent
//! to cast it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("FreestriderCommando.FreestriderCommandoCondition", not_cast_or_free)];

/// not entering as a resolving spell, or none of the spell's mana paid
fn not_cast_or_free(game: &Game, _: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    !o.cast || o.mana_spent == 0
}
