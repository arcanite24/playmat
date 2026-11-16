//! Rona, Disciple of Gix: "When Rona enters, you may exile target historic card from your graveyard. You may cast
//! nonland cards exiled with Rona. {4}, {T}: Exile the top card of your library."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("RonaDiscipleOfGix.RonaDiscipleOfGixPlayNonLandEffect", AsThough::Play(play))];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: you may cast a nonland card in this permanent's exile zone (as it is now)
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    let zcc = game.obj(cx.source).zcc;
    let with = game.exiled_for_source.iter().any(|(s, z, c)| *s == cx.source && *z == zcc && *c == id);
    (seat == cx.controller && with && game.obj(id).zone == Zone::Exile && !game.has_type(id, CardType::Land))
        .then(PlayWay::default)
}
