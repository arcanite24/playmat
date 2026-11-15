//! Ebondeath, Dracolich: "Flash. Flying. Ebondeath, Dracolich enters tapped. You may cast Ebondeath, Dracolich from
//! your graveyard if a creature not named Ebondeath, Dracolich died this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("EbondeathDracolich.EbondeathDracolichEffect", AsThough::Play(play)).in_any_zone()];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, from its controller's graveyard, once a creature not named Ebondeath,
/// Dracolich died this turn (`EbondeathDracolichWatcher`)
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    (id == cx.source
        && seat == cx.controller
        && game.obj(id).zone == Zone::Graveyard
        && game
            .watch
            .left_battlefield
            .iter()
            .any(|l| l.died && l.creature && game.obj(l.id).name() != "Ebondeath, Dracolich"))
    .then(PlayWay::default)
}
