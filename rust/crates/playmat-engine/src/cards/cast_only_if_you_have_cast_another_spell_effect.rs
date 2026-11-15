//! `CastOnlyIfYouHaveCastAnotherSpellEffect` (`Zone.ALL`): "Cast this spell only if you've cast another spell this
//! turn." (Hewed Stone Retainers, Illusory Angel): `CAST_SPELL` of this card while its controller has cast no spell
//! this turn (`CastSpellLastTurnWatcher`).

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("CastOnlyIfYouHaveCastAnotherSpellEffect", Rule::CantCast(cant_cast)).in_any_zone()];

fn cant_cast(game: &Game, cx: &StaticCx, _seat: Seat, id: ObjId, _: &CardDef) -> bool {
    id == cx.source && game.player(cx.controller).spells_cast == 0
}
