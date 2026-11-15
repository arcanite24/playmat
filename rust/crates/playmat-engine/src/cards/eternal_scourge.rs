//! Eternal Scourge: "You may cast Eternal Scourge from exile. When Eternal Scourge becomes the target of a spell or
//! ability an opponent controls, exile Eternal Scourge."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("EternalScourge.EternalScourgePlayEffect", AsThough::Play(play)).in_any_zone()];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, by its owner, from exile
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    let o = game.obj(id);
    (id == cx.source && o.owner == seat && o.zone == Zone::Exile).then(PlayWay::default)
}
