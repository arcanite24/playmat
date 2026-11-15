//! Bontu the Glorified: "Bontu the Glorified can't attack or block unless a creature died under your control this
//! turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "BontuTheGlorified.BontuTheGlorifiedRestrictionEffect",
    applies,
    &[Restrict::CantAttack(no_attack), Restrict::CantBlock(no_block)],
)];

/// itself, while `CreaturesDiedWatcher` has no creature of yours dying this turn
fn applies(game: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    permanent == cx.source && !game.watch.died.contains(&cx.controller)
}

fn no_attack(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

fn no_block(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}
