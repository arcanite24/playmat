//! Theater of Horrors: "During your turn, if an opponent lost life this turn, you may play lands and cast spells from
//! among cards exiled with Theater of Horrors."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("TheaterOfHorrors.TheaterOfHorrorsCastEffect", AsThough::Play(play))];

/// `PLAY_FROM_NOT_OWN_HAND_ZONE`: a card in this one's exile zone (`getCardExileZoneId`: as it is now), by its
/// controller during their turn, once an opponent lost life this turn (`PlayerLostLifeWatcher.getAllOppLifeLost`)
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    let you = cx.controller;
    let zcc = game.obj(cx.source).zcc;
    (seat == you
        && game.active == Some(you)
        && game.obj(id).zone == Zone::Exile
        && (0..game.seats()).any(|p| p != you && game.player(p).life_lost > 0)
        && game.exiled_for_source.iter().any(|(s, z, o)| *s == cx.source && *z == zcc && *o == id))
        .then(PlayWay::default)
}
