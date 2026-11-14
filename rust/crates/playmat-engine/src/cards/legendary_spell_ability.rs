//! `LegendarySpellAbility` (Jaya's Immolating Inferno, Kamahl's Druidic Vow): "(You may cast a legendary sorcery only
//! if you control a legendary creature or planeswalker.)" A static ability in every zone with
//! `LegendarySpellAbilityCheckEffect`, its argument (instant or sorcery) only for its text.

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("LegendarySpellAbilityCheckEffect", Rule::CantCast(cant_cast)).args(&["text"]).in_any_zone()];

/// `CAST_SPELL` of this card, while its caster controls no legendary creature or planeswalker
fn cant_cast(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> bool {
    id == cx.source
        && !game.battlefield.iter().any(|p| {
            game.obj(*p).controller == seat
                && game.obj(*p).def.supertypes.iter().any(|t| t == "LEGENDARY")
                && (game.has_type(*p, CardType::Creature) || game.has_type(*p, CardType::Planeswalker))
        })
}
