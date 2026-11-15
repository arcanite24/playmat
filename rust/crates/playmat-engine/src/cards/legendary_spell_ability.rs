//! `LegendarySpellAbility`: "(You may cast a legendary sorcery only if you control a legendary creature or
//! planeswalker.)" (Urza's Ruinous Blast, Yawgmoth's Vile Offering, Kamahl's Druidic Vow…): its
//! `LegendarySpellAbilityCheckEffect` on `CAST_SPELL`, in every zone.

use super::*;
use crate::card::{CardDef, CardType};

pub const HANDLERS: &[Handler] =
    &[Handler::rule("LegendarySpellAbilityCheckEffect", Rule::CantCast(cant_cast)).in_any_zone()];

/// The card's own spell, while its caster controls no legendary creature or planeswalker
/// (`containsControlled(filter, source, game, 1)`)
fn cant_cast(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> bool {
    id == cx.source
        && !game.battlefield.iter().any(|o| {
            game.obj(*o).controller == seat
                && game.obj(*o).def.supertypes.iter().any(|s| s == "LEGENDARY")
                && (game.has_type(*o, CardType::Creature) || game.has_type(*o, CardType::Planeswalker))
        })
}
