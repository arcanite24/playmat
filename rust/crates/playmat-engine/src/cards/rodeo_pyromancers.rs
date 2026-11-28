//! Rodeo Pyromancers: "Whenever you cast your first spell each turn, add {R}{R}."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "RodeoPyromancers.RodeoPyromancersTriggeredAbility",
    "new SpellCastControllerTriggeredAbility(new BasicManaEffect(Mana.RedMana(2)), false)",
    first_spell,
)];

/// `SpellsCastWatcher`: the spell this trigger saw is the first one you cast this turn
fn first_spell(game: &Game, item: &StackItem) -> bool {
    let Some(spell) = item.cast_spell else { return false };
    game.watch.spells.iter().find(|s| s.caster == item.controller).map(|s| s.card) == Some(spell)
}
