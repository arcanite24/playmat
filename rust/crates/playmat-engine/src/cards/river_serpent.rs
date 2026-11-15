//! River Serpent: "River Serpent can't attack unless there are five or more cards in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::restriction("RiverSerpent.RiverSerpentEffect", applies, &[Restrict::CantAttack(always)])];

/// the source itself, with fewer than five cards in its controller's graveyard
fn applies(game: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    permanent == cx.source && game.player(cx.controller).graveyard.len() < 5
}

fn always(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}
