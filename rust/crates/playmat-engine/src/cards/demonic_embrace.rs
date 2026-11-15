//! Demonic Embrace: "Enchant creature. Enchanted creature gets +3/+1, has flying, and is a Demon in addition to its
//! other types. You may cast Demonic Embrace from your graveyard by paying 3 life and discarding a card in addition
//! to paying its other costs."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("DemonicEmbrace.DemonicEmbracePlayEffect", AsThough::Play(play)).in_any_zone()];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, from its controller's graveyard, for {1}{B}{B} with `PayLifeCost(3)`
/// and `DiscardCardCost()` (`setCastSourceIdWithAlternateMana`)
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    (id == cx.source && seat == cx.controller && game.obj(id).zone == Zone::Graveyard).then(|| PlayWay {
        mana: ManaCost::parse("{1}{B}{B}").ok(),
        costs: vec![
            CostDef::PayLifeCost { amount: 3 },
            CostDef::DiscardCardCost { filter: crate::card::Filter::default(), amount: 1 },
        ],
    })
}
