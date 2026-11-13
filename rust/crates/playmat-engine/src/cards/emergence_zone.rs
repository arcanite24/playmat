//! Emergence Zone: "{1}, {T}, Sacrifice Emergence Zone: You may cast spells this turn as though they had flash." A
//! `CastAsThoughItHadFlashAllEffect(Duration.EndOfTurn, filter)` resolving: until the turn's cleanup, its
//! controller's spells of the filter (checked against the face cast).

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("EmergenceZone.CastAsThoughItHadFlashAllEffect", AsThough::Flash(flash))
        .args(&["one_of:Duration.EndOfTurn", "filter"])];

fn flash(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, face: &CardDef) -> bool {
    seat == cx.controller && cx.filters.first().is_some_and(|f| face_matches(game, f, seat, cx.source, id, face))
}
