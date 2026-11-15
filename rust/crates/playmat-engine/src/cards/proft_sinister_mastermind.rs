//! Proft, Sinister Mastermind: "Threshold — You can't cast this spell unless there are seven or more cards in your
//! graveyard."

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("ProftSinisterMastermind.ProftSinisterMastermindEffect", Rule::CantCast(cant_cast)).in_any_zone()];

/// its own spell, without `ThresholdCondition` (seven or more cards in your graveyard)
fn cant_cast(game: &Game, cx: &StaticCx, _: Seat, id: ObjId, _: &CardDef) -> bool {
    id == cx.source && game.player(cx.controller).graveyard.len() < 7
}
