//! Obelisk of Urd: "Convoke. As Obelisk of Urd enters, choose a creature type. Creatures you control of the chosen
//! type get +2/+2."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("ObeliskOfUrd.ObeliskOfUrdBoostEffect", affects, Layer::ModifyPt(plus_two))];

/// creatures you control of the type the Obelisk chose (none chosen: none)
fn affects(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.obj(id).controller == cx.controller
        && game.has_type(id, CardType::Creature)
        && game.obj(cx.source).chosen_type.as_ref().is_some_and(|t| game.has_subtype(id, t))
}

fn plus_two(_: &Game, _: &StaticCx, _: ObjId) -> (i32, i32) {
    (2, 2)
}
