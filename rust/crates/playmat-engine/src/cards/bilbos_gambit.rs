//! Bilbo's Gambit: "Gift a Treasure. Return target spell to its owner's hand. If the gift was promised, players
//! can't cast spells this turn."

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] = &[Handler::rule("BilbosGambit.BilbosGambitEffect", Rule::CantCast(any_spell))];

/// `CAST_SPELL`, by anyone, until the turn's end
fn any_spell(_: &Game, _: &StaticCx, _: Seat, _: ObjId, _: &CardDef) -> bool {
    true
}
