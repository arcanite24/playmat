//! `CantBlockUnlessYouControlSourceEffect(filter)`: "{this} can't block unless you control [filter]" (Felhide
//! Brawler, Howlpack Wolf, Marauding Boneslasher): the source, while no permanent of the filter is on the battlefield
//! for its controller (`Battlefield.count(filter, controllerId, source)`).

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "CantBlockUnlessYouControlSourceEffect",
    applies,
    &[Restrict::CantBlock(no_block)],
)
.args(&["filter"])];

fn applies(game: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    permanent == cx.source
        && !game.battlefield.iter().any(|c| game.obj(*c).zone == Zone::Battlefield && cx.matches(game, 0, *c))
}

fn no_block(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}
