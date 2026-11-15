//! Oathsworn Vampire: "Oathsworn Vampire enters the battlefield tapped. You may cast Oathsworn Vampire from your
//! graveyard if you gained life this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("OathswornVampire.OathswornVampirePlayEffect", AsThough::Play(play)).in_any_zone()];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, from its controller's graveyard, once they gained life this turn
/// (`PlayerGainedLifeWatcher`)
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    (id == cx.source
        && seat == cx.controller
        && game.obj(id).zone == Zone::Graveyard
        && game.player(cx.controller).life_gained > 0)
        .then(PlayWay::default)
}
