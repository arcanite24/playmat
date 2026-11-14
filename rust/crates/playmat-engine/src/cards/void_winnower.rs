//! Void Winnower: "Your opponents can't cast spells with even mana values. (Zero is even.) Your opponents can't
//! block with creatures with even mana values."

use super::*;
use crate::card::{CardDef, CardType};

pub const HANDLERS: &[Handler] = &[
    Handler::rule("VoidWinnower.VoidWinnowerCantCastEffect", Rule::CantCast(cant_cast)),
    Handler::restriction("VoidWinnower.VoidWinnowerCantBlockEffect", even_opponents_creature, &[Restrict::CantBlock(no_block)]),
];

/// `CAST_SPELL_LATE`: an opponent's spell with an even mana value (the face cast)
fn cant_cast(_game: &Game, cx: &StaticCx, seat: Seat, _id: ObjId, face: &CardDef) -> bool {
    cx.opponent(seat) && face.mana_value().is_multiple_of(2)
}

/// a creature an opponent controls with an even mana value
fn even_opponents_creature(game: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    let o = game.obj(permanent);
    cx.opponent(o.controller) && o.def.mana_value().is_multiple_of(2) && game.has_type(permanent, CardType::Creature)
}

fn no_block(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}
