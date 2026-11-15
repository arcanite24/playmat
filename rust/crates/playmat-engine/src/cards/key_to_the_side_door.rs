//! Key to the Side-Door: "{2}, {T}: Target creature can't be blocked this turn. {1}, {T}, Discard a legendary card
//! with the same name as a legendary permanent you control: Draw two cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("KeyToTheSideDoor.KeyToTheSideDoorPredicate", shares_name)];

/// a legendary permanent the player controls has the card's name (`CardUtil.haveSameNames`: no empty names)
fn shares_name(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let name = game.obj(id).name();
    !name.is_empty()
        && game.battlefield.iter().any(|p| {
            let o = game.obj(*p);
            o.controller == you && o.def.supertypes.iter().any(|s| s == "LEGENDARY") && o.name() == name
        })
}
