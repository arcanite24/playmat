//! Scourge of Nel Toth: "You may cast Scourge of Nel Toth from your graveyard by paying {B}{B} and sacrificing two
//! creatures rather than paying its mana cost."

use super::*;
use crate::card::Filter;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("ScourgeOfNelToth.ScourgeOfNelTothPlayEffect", AsThough::Play(play)).in_any_zone()];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, from its controller's graveyard, for {B}{B} with
/// `SacrificeTargetCost(2, creatures)` (`setCastSourceIdWithAlternateMana`)
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    (id == cx.source && seat == cx.controller && game.obj(id).zone == Zone::Graveyard).then(|| PlayWay {
        mana: ManaCost::parse("{B}{B}").ok(),
        costs: vec![CostDef::SacrificeTargetCost { filter: Filter::creatures(), amount: 2 }],
    })
}
