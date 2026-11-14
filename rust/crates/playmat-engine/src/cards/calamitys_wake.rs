//! Calamity's Wake: "Exile all graveyards. Players can't cast noncreature spells this turn. Exile Calamity's Wake."

use super::*;
use crate::card::{CardDef, CardType};

pub const HANDLERS: &[Handler] = &[Handler::rule("CalamitysWake.CalamitysWakeEffect", Rule::CantCast(cant_cast))];

/// `CAST_SPELL` of a card that isn't a creature, by any player, this turn
fn cant_cast(_game: &Game, _cx: &StaticCx, _seat: Seat, _id: ObjId, face: &CardDef) -> bool {
    !face.is(CardType::Creature)
}
