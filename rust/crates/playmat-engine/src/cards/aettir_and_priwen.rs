//! Aettir and Priwen: "Equipped creature has base power and toughness X/X, where X is your life total."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("AettirAndPriwen.AettirAndPriwenEffect", equipped, Layer::SetPt(life_total))];

/// the permanent the source is attached to
fn equipped(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.attached(cx.source) == Some(id)
}

/// X/X, X the controller's life total (`setModifiedBaseValue`, layer 7b)
fn life_total(game: &Game, cx: &StaticCx, _: ObjId) -> (i32, i32) {
    let life = game.player(cx.controller).life;
    (life, life)
}
