//! Wickerfolk Indomitable: "You may cast this card from your graveyard by paying 2 life and sacrificing an artifact or
//! creature in addition to paying its other costs."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::as_though(
    "WickerfolkIndomitable.WickerfolkIndomitableGraveyardEffect",
    AsThough::Play(play),
)
.in_any_zone()];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, from its controller's graveyard, for {3}{B} with `PayLifeCost(2)` and
/// `SacrificeTargetCost(FILTER_CONTROLLED_PERMANENT_ARTIFACT_OR_CREATURE)` (`setCastSourceIdWithAlternateMana`)
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    (id == cx.source && seat == cx.controller && game.obj(id).zone == Zone::Graveyard).then(|| PlayWay {
        mana: ManaCost::parse("{3}{B}").ok(),
        costs: vec![
            CostDef::PayLifeCost { amount: 2 },
            CostDef::SacrificeTargetCost {
                filter: Filter::of(vec![
                    Pred::Controller { who: "you".into() },
                    Pred::Or { xs: vec![Pred::Type { t: CardType::Artifact }, Pred::Type { t: CardType::Creature }] },
                ]),
                amount: 1,
            },
        ],
    })
}
