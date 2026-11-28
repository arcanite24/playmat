//! Skyblinder Staff: "Equipped creature gets +1/+0 and can't be blocked by creatures with flying."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "SkyblinderStaff.CantBeBlockedByCreaturesWithFlyingAttachedEffect",
    applies,
    &[Restrict::CantBeBlocked(by)],
)];

fn applies(game: &Game, cx: &StaticCx, attacker: ObjId) -> bool {
    game.attached(cx.source) == Some(attacker)
}

fn by(game: &Game, _: &StaticCx, _: ObjId, blocker: ObjId) -> bool {
    game.has_ability(blocker, &AbilityDef::FlyingAbility)
}
