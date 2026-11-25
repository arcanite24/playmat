//! Experimental Frenzy: "You may look at the top card of your library any time. You may play lands and cast spells
//! from the top of your library. You can't play cards from your hand. {3}{R}: Destroy Experimental Frenzy."

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] = &[Handler::rule(
    "ExperimentalFrenzy.ExperimentalFrenzyRestrictionEffect",
    Rule::Several(&[Rule::CantCast(cant_cast_from_hand), Rule::CantPlayLandsFromHand(you)]),
)];

/// `CAST_SPELL` by its controller of a card in their hand
fn cant_cast_from_hand(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> bool {
    seat == cx.controller && game.obj(id).zone == Zone::Hand
}

/// `PLAY_LAND` by its controller from their hand
fn you(_: &Game, cx: &StaticCx, seat: Seat) -> bool {
    seat == cx.controller
}
