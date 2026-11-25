//! `OpponentsCantCastChosenUntilNextTurnEffect`: "Until your next turn, your opponents can't cast spells with the
//! chosen name" (Comply, Academic Probation). Java checks `CAST_SPELL_LATE`, once the spell's modes and targets are
//! chosen, against the name `ChooseACardNameEffect` kept for the source (which outlives the spell).

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] = &[Handler::rule(
    "OpponentsCantCastChosenUntilNextTurnEffect",
    Rule::CantCastOnStack(cant_cast),
)
.until_your_next_turn()];

/// an opponent's spell with the name chosen for this source last (`CardUtil.haveSameNames`: a split card by either
/// half's name or its full one)
fn cant_cast(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, face: &CardDef) -> bool {
    if !cx.opponent(seat) {
        return false;
    }
    let Some((_, name)) = game.names_chosen.iter().rev().find(|(s, _)| *s == cx.source) else { return false };
    !name.is_empty() && (face.name == *name || game.obj(id).def.name == *name)
}
