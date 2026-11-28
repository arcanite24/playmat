//! Gravecrawler: "Gravecrawler can't block. You may cast Gravecrawler from your graveyard as long as you control a
//! Zombie."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("Gravecrawler.GravecrawlerPlayEffect", AsThough::Play(play)).in_any_zone()];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, from its controller's graveyard, for its own costs, while they control
/// a Zombie creature
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    let zombie = game.battlefield.iter().any(|o| {
        game.obj(*o).controller == cx.controller && game.has_type(*o, CardType::Creature) && game.has_subtype(*o, "Zombie")
    });
    (id == cx.source && seat == cx.controller && game.obj(id).zone == Zone::Graveyard && zombie)
        .then(PlayWay::default)
}
