//! Rubblebelt Braggart: "Whenever this creature attacks, if it's not suspected, you may suspect it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("RubblebeltBraggart.RubblebeltBraggartCondition", not_suspected)];

/// the source is still on the battlefield and isn't suspected
fn not_suspected(game: &Game, _: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    o.zone == Zone::Battlefield && !o.suspected()
}
