//! `CanBlockOnlyFlyingAttachedEffect(AttachmentType)`: "Enchanted creature can block only creatures with flying."
//! (Stratus Walk)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "CanBlockOnlyFlyingAttachedEffect",
    applies,
    &[Restrict::CantBlock(cant_block)],
)
.args(&["one_of:AttachmentType.AURA|AttachmentType.EQUIPMENT"])];

fn applies(game: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    game.attached(cx.source) == Some(permanent)
}

fn cant_block(game: &Game, _: &StaticCx, _: ObjId, attacker: ObjId) -> bool {
    !game.has_ability(attacker, &AbilityDef::FlyingAbility)
}
