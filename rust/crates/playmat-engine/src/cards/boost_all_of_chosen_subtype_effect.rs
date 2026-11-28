//! `BoostAllOfChosenSubtypeEffect(power, toughness, WhileOnBattlefield, filter, excludeSource)`: "creatures you
//! control of the chosen type get +1/+1" (Patchwork Banner, Chronicle of Victory). Layer 7c, the filter looked at
//! each time, and of the creature type the source chose as it entered (none chosen: no boost).

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("BoostAllOfChosenSubtypeEffect", affects, Layer::ModifyPt(boost)).args(&[
        "text",
        "text",
        "while_on_battlefield",
        "filter",
        "text",
    ])];

fn affects(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    !(cx.arg_bool(4) && id == cx.source)
        && cx.matches(game, 0, id)
        && game.obj(cx.source).chosen_type.as_ref().is_some_and(|t| game.has_subtype(id, t))
}

fn boost(_: &Game, cx: &StaticCx, _: ObjId) -> (i32, i32) {
    (cx.arg_int(0).unwrap_or(0), cx.arg_int(1).unwrap_or(0))
}
