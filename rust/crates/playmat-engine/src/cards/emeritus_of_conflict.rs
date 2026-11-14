//! Emeritus of Conflict: "Whenever you cast your third spell each turn, this creature becomes prepared."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "EmeritusOfConflict.EmeritusOfConflictTriggeredAbility",
    "new SpellCastControllerTriggeredAbility(new BecomePreparedSourceEffect(), false)",
    third_spell,
)];

/// `SpellsCastWatcher.getSpellsCastThisTurn(you).size() == 3` as the spell is cast: the spell this trigger saw is
/// your third this turn (asked of that spell, so it holds again as the trigger resolves)
fn third_spell(game: &Game, item: &StackItem) -> bool {
    let you = item.controller;
    let Some(spell) = item.cast_spell else { return false };
    game.watch.spells.iter().filter(|s| s.caster == you).nth(2).is_some_and(|s| s.card == spell)
}
