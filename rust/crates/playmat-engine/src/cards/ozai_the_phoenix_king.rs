//! Ozai, the Phoenix King: "Trample, firebending 4, haste. If you would lose unspent mana, that mana becomes red
//! instead. Ozai has flying and indestructible as long as you have six or more unspent mana."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[
    Handler::rule("OzaiThePhoenixKing.OzaiThePhoenixKingManaEffect", Rule::ManaBecomes(red)),
    Handler::continuous("OzaiThePhoenixKing.OzaiThePhoenixKingBoostEffect", itself_with_six, Layer::Ability(gives)),
];

/// its controller's unspent mana becomes red (`setManaBecomesRed`)
fn red(_: &Game, cx: &StaticCx, seat: Seat) -> Option<usize> {
    (seat == cx.controller).then_some(Color::R.index())
}

/// Ozai, while its controller has six or more mana in their pool
fn itself_with_six(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source && game.player(cx.controller).mana_pool.total() >= 6
}

fn gives(_: &Game, _: &StaticCx, _: ObjId) -> Vec<AbilityDef> {
    vec![AbilityDef::FlyingAbility, AbilityDef::IndestructibleAbility]
}
