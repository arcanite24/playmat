//! Liberator, Urza's Battlethopter: "Whenever you cast a spell, if the amount of mana spent to cast that spell is
//! greater than Liberator's power, put a +1/+1 counter on Liberator."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("LiberatorUrzasBattlethopter.LiberatorUrzasBattlethopterCondition", more_than_power)];

/// The trigger being checked or resolving: a spell trigger of this source, its event's spell (`getValue`), or
/// none while the engine first matches the event (it's checked again once the trigger has its details)
pub(super) fn trigger_item(game: &Game, source: ObjId) -> Option<&StackItem> {
    game.checking_item.as_ref().or(game.resolving_item.as_ref()).filter(|i| i.card == source && i.ability.is_some())
}

/// the source still on the battlefield, and the mana spent to cast the spell is greater than its power
fn more_than_power(game: &Game, _: Seat, source: ObjId) -> bool {
    let Some(item) = trigger_item(game, source) else { return true };
    let Some(spell) = item.cast_spell else { return false };
    game.obj(source).zone == Zone::Battlefield && game.obj(spell).mana_spent as i32 > game.power_toughness(source).0
}
