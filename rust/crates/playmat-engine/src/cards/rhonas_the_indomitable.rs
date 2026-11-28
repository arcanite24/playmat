//! Rhonas the Indomitable: "Rhonas the Indomitable can't attack or block unless you control another creature with
//! power 4 or greater."

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "RhonasTheIndomitable.RhonasTheIndomitableRestrictionEffect",
    applies,
    &[Restrict::CantAttack(no_attack), Restrict::CantBlock(no_block)],
)];

/// the source itself, while its controller controls no other creature with power greater than 3
fn applies(game: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    permanent == cx.source
        && !game.battlefield.iter().any(|c| {
            *c != cx.source
                && game.obj(*c).controller == cx.controller
                && game.has_type(*c, CardType::Creature)
                && game.power_toughness(*c).0 > 3
        })
}

fn no_attack(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

fn no_block(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}
