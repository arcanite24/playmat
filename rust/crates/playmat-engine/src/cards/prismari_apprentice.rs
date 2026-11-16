//! Prismari Apprentice: "Magecraft — Whenever you cast or copy an instant or sorcery spell, this creature can't be
//! blocked this turn. If that spell has mana value 5 or greater, put a +1/+1 counter on it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("PrismariApprentice.PrismariApprenticeCondition", five_or_more)];

/// the spell the trigger saw (`MagecraftAbility.SPELL_KEY`) has mana value 5 or more: a cast spell's is the
/// trigger's X (its mana value as cast, X included); a copy's, its card's
fn five_or_more(game: &Game, _: Seat, _: ObjId) -> bool {
    let Some(item) = game.resolving_item.as_ref() else { return false };
    let Some(spell) = item.cast_spell else { return false };
    let mv = if item.x > 0 { item.x } else { game.obj(spell).def.mana_value() as i32 };
    mv >= 5
}
