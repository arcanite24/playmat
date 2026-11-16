//! Aquatic Alchemist // Bubble Up: "Whenever you cast your first instant or sorcery spell each turn, Aquatic
//! Alchemist gets +2/+0 until end of turn." Bubble Up: "Put target instant or sorcery card from your graveyard on top
//! of your library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "AquaticAlchemist.AquaticAlchemistTriggeredAbility",
    "new SpellCastControllerTriggeredAbility(new BoostSourceEffect(2, 0, Duration.EndOfTurn), StaticFilters.FILTER_SPELL_AN_INSTANT_OR_SORCERY, false)",
    first_instant_or_sorcery,
)];

/// `SpellsCastWatcher`: of the instants and sorceries you cast this turn, the spell cast is the only one (as it
/// triggers; as it resolves, the first: those cast since don't undo it)
fn first_instant_or_sorcery(game: &Game, item: &StackItem) -> bool {
    let Some(spell) = item.cast_spell else { return false };
    game
        .watch
        .spells
        .iter()
        .filter(|s| s.caster == item.controller)
        .map(|s| s.card)
        .find(|c| game.obj(*c).is(CardType::Instant) || game.obj(*c).is(CardType::Sorcery))
        == Some(spell)
}
