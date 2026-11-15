//! Dragon Hunter: "Protection from Dragons. Dragon Hunter can block Dragons as though it had reach."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::as_though(
    "DragonHunter.CanBlockDragonsAsThoughtIthadReachEffect",
    AsThough::BlockDragon(itself),
)
.args(&["while_on_battlefield"])];

/// `BLOCK_DRAGON` for this creature itself
fn itself(_: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
}
