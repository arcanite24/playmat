//! `MayCastFromGraveyardSourceAbility`'s `MayCastFromGraveyardEffect`: "You may cast this card from your graveyard."
//! (Hundred-Battle Veteran)

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("MayCastFromGraveyardEffect", AsThough::Play(play)).in_any_zone()];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, from its controller's graveyard, for its own mana cost
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    (id == cx.source && seat == cx.controller && game.obj(id).zone == Zone::Graveyard).then(PlayWay::default)
}
