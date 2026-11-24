//! Rona, Sheoldred's Faithful: "Whenever you cast an instant or sorcery spell, each opponent loses 1 life. You may
//! cast Rona from your graveyard by discarding two cards in addition to paying its other costs."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::as_though(
    "RonaSheoldredsFaithful.RonaSheoldredsFaithfulEffect",
    AsThough::Play(play),
)
.in_any_zone()];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, from its controller's graveyard, for {1}{U}{B}{B} with
/// `DiscardTargetCost(TargetCardInHand(2, …))` (`setCastSourceIdWithAlternateMana`)
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    (id == cx.source && seat == cx.controller && game.obj(id).zone == Zone::Graveyard).then(|| PlayWay {
        mana: ManaCost::parse("{1}{U}{B}{B}").ok(),
        costs: vec![CostDef::DiscardCardCost { filter: crate::card::Filter::default(), amount: 2, different_names: false }],
    })
}
