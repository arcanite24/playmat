//! Festival of Embers: "During your turn, you may cast instant and sorcery spells from your graveyard by paying 1 life
//! in addition to their other costs."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("FestivalOfEmbers.FestivalOfEmbersCastEffect", AsThough::Play(play))];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: an instant or sorcery card you own in your graveyard, during your turn, for its mana
/// cost (`setCastSourceIdWithAlternateMana`, so its mana isn't counted in the playable check) and 1 life
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, face: &CardDef) -> Option<PlayWay> {
    let o = game.obj(id);
    (seat == cx.controller
        && game.active == Some(seat)
        && o.owner == seat
        && o.zone == Zone::Graveyard
        && (o.is(CardType::Instant) || o.is(CardType::Sorcery)))
    .then(|| PlayWay { mana: Some(face.cost.clone()), costs: vec![CostDef::PayLifeCost { amount: 1 }] })
}
