//! Squee, Dubious Monarch: "Haste. Whenever Squee attacks, create a 1/1 red Goblin creature token that's tapped and
//! attacking. You may cast Squee from your graveyard by paying {3}{R} and exiling four other cards from your graveyard
//! rather than paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("SqueeDubiousMonarch.SqueeDubiousMonarchEffect", AsThough::Play(play)).in_any_zone()];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, from its controller's graveyard, for {3}{R} with
/// `ExileFromGraveCost(TargetCardInYourGraveyard(4, other cards))` (`setCastSourceIdWithAlternateMana`)
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    (id == cx.source && seat == cx.controller && game.obj(id).zone == Zone::Graveyard).then(|| PlayWay {
        mana: ManaCost::parse("{3}{R}").ok(),
        costs: vec![CostDef::ExileFromGraveCost { amount: 4, filter: Filter::default() }],
    })
}
