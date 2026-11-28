//! Complete the Circuit: "Convoke. You may cast sorcery spells this turn as though they had flash. When you next
//! cast an instant or sorcery spell this turn, copy that spell twice. You may choose new targets for the copies."
//! Its `CastAsThoughItHadFlashAllEffect(Duration.EndOfTurn, filter)` as Emergence Zone's; the delayed copy is the
//! engine's `CreateCopyNextSpellDelayedEffect` (transpile.py).

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("CompleteTheCircuit.CastAsThoughItHadFlashAllEffect", AsThough::Flash(flash))
        .args(&["one_of:Duration.EndOfTurn", "filter"])];

fn flash(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, face: &CardDef) -> bool {
    seat == cx.controller && cx.filters.first().is_some_and(|f| face_matches(game, f, seat, cx.source, id, face))
}
