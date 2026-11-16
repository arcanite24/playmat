//! Ixalan's Binding: "When Ixalan's Binding enters the battlefield, exile target nonland permanent an opponent
//! controls until Ixalan's Binding leaves the battlefield. Your opponents can't cast spells with the same name as the
//! exiled card."

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("IxalansBinding.IxalansBindingReplacementEffect", Rule::CantCastOnStack(cant_cast))];

/// `CAST_SPELL_LATE`: an opponent's spell with the name of a card still exiled until this permanent (as it is now)
/// leaves (`CardUtil.haveSameNames`: no name matches nothing)
fn cant_cast(game: &Game, cx: &StaticCx, seat: Seat, _id: ObjId, face: &CardDef) -> bool {
    if !cx.opponent(seat) || face.name.is_empty() || game.obj(cx.source).zone != Zone::Battlefield {
        return false;
    }
    let zcc = game.obj(cx.source).zcc;
    game.exiled_until_leaves.iter().any(|(s, z, c)| {
        *s == cx.source && *z == zcc && game.obj(*c).zone == Zone::Exile && game.obj(*c).name() == face.name
    })
}
