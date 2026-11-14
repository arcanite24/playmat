//! Squee, the Immortal: "You may cast Squee, the Immortal from your graveyard or from exile."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("SqueeTheImmortal.SqueePlayEffect", AsThough::Play(play)).in_any_zone()];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, by its owner, from a graveyard or exile
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    let o = game.obj(id);
    (id == cx.source && o.owner == seat && matches!(o.zone, Zone::Graveyard | Zone::Exile)).then(PlayWay::default)
}
