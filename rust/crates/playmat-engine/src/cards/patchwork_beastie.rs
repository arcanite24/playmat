//! Patchwork Beastie: "Delirium — This creature can't attack or block unless there are four or more card types among
//! cards in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "PatchworkBeastie.PatchworkBeastieEffect",
    applies,
    &[Restrict::CantAttack(always), Restrict::CantBlock(always_block)],
)];

/// the source itself, without delirium
fn applies(game: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    permanent == cx.source
        && !game.condition_holds(Some(&crate::card::Condition::Delirium), cx.controller, cx.source)
}

fn always(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

fn always_block(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}
