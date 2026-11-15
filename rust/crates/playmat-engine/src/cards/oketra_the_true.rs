//! Oketra the True: "Double strike, indestructible. Oketra the True can't attack or block unless you control at least
//! three other creatures. {3}{W}: Create a 1/1 white Warrior creature token with vigilance."

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "OketraTheTrue.OketraTheTrueRestrictionEffect",
    applies,
    &[Restrict::CantAttack(no_attack), Restrict::CantBlock(no_block)],
)];

/// the source itself, while its controller controls fewer than four creatures (itself counted)
fn applies(game: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    permanent == cx.source
        && game
            .battlefield
            .iter()
            .filter(|c| game.obj(**c).controller == cx.controller && game.has_type(**c, CardType::Creature))
            .count()
            < 4
}

fn no_attack(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

fn no_block(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}
