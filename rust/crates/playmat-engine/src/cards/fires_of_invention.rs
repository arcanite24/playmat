//! Fires of Invention: "You can cast spells only during your turn and you can cast no more than two spells each
//! turn. You may cast spells with mana value less than or equal to the number of lands you control without paying
//! their mana costs."

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("FiresOfInvention.FiresOfInventionCastEffect", Rule::CantCast(cant_cast))];

/// `CAST_SPELL` by its controller once they've cast two spells this turn (`CastSpellLastTurnWatcher`), or during
/// another player's turn
fn cant_cast(game: &Game, cx: &StaticCx, seat: Seat, _: ObjId, _: &CardDef) -> bool {
    seat == cx.controller && (game.player(seat).spells_cast > 1 || game.active != Some(seat))
}
