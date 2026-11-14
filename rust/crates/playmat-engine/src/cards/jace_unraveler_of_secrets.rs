//! Jace, Unraveler of Secrets' emblem: "Whenever an opponent casts their first spell each turn, counter that spell."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "JaceUnravelerOfSecretsTriggeredAbility",
    "new SpellCastOpponentTriggeredAbility(Zone.BATTLEFIELD, $0, StaticFilters.FILTER_SPELL, $1, SetTargetPointer.SPELL)",
    first_spell,
)];

/// the caster's first spell this turn (`SpellsCastWatcher`)
fn first_spell(game: &Game, item: &StackItem) -> bool {
    let Some(s) = item.cast_spell else { return false };
    game.player(game.obj(s).controller).spells_cast == 1
}
