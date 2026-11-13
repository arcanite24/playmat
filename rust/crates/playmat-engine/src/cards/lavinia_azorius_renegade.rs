//! Lavinia, Azorius Renegade: "Each opponent can't cast noncreature spells with mana value greater than the number of
//! lands that player controls." (`CAST_SPELL`: the card's mana value, checked in the playable list and as it's cast)

use super::*;
use crate::card::{CardDef, CardType};

pub const HANDLERS: &[Handler] =
    &[Handler::rule("LaviniaAzoriusRenegade.LaviniaAzoriusRenegadeReplacementEffect", Rule::CantCast(cant_cast))];

fn cant_cast(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> bool {
    if !cx.opponent(seat) {
        return false;
    }
    let lands = game
        .battlefield
        .iter()
        .filter(|l| game.obj(**l).controller == seat && game.has_type(**l, CardType::Land))
        .count() as u32;
    // `game.getCard(event.getSourceId())`: the card, whatever face is cast
    let card = &game.obj(id).def;
    !card.is(CardType::Creature) && card.mana_value() > lands
}
