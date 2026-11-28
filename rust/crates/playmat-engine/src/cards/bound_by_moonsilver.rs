//! Bound by Moonsilver: "Enchant creature. Enchanted creature can't attack, block, or transform. Sacrifice another
//! permanent: Attach Bound by Moonsilver to target creature. Activate only as a sorcery and only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "BoundByMoonsilver.BoundByMoonsilverEffect",
    enchanted,
    &[Restrict::CantAttack(always), Restrict::CantBlock(never_blocks), Restrict::CantTransform(always)],
)];

/// `permanent.getAttachments().contains(source)`
fn enchanted(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.attached(cx.source) == Some(id)
}

fn always(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

fn never_blocks(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}
