//! Mirror Box: "The \"legend rule\" doesn't apply to permanents you control. Each legendary creature you control gets
//! +1/+1. Each nontoken creature you control gets +1/+1 for each other creature you control with the same name as
//! that creature."

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("MirrorBox.MirrorBoxBoostEffect", yours_nontoken, Layer::ModifyPt(same_names))];

/// each nontoken creature you control
fn yours_nontoken(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    let o = game.obj(id);
    o.controller == cx.controller && !o.token && game.has_type(id, CardType::Creature)
}

/// +1/+1 for each other creature you control with its name (`CardUtil.haveSameNames`: a nameless one has none)
fn same_names(game: &Game, cx: &StaticCx, id: ObjId) -> (i32, i32) {
    let name = &game.obj(id).def.name;
    if name.is_empty() {
        return (0, 0);
    }
    let n = game
        .battlefield
        .iter()
        .filter(|c| {
            **c != id
                && game.obj(**c).controller == cx.controller
                && game.has_type(**c, CardType::Creature)
                && game.obj(**c).def.name == *name
        })
        .count() as i32;
    (n, n)
}
