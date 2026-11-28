//! The Queen of Dale: "Whenever an opponent casts their first noncreature spell each turn, you recruit."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "TheQueenOfDale.TheQueenOfDaleTriggeredAbility",
    "new SpellCastOpponentTriggeredAbility(new RecruitEffect(), StaticFilters.FILTER_SPELL_A_NON_CREATURE, false)",
    check,
)];

/// `SpellsCastWatcher`: it's the first noncreature spell that opponent cast this turn (a duel: the other seat)
fn check(game: &Game, t: &StackItem) -> bool {
    (0..game.players.len()).any(|p| p != t.controller && game.player(p).noncreature_cast == 1)
}
