//! Plan for All Outcomes: "Whenever you cast your first noncreature spell each turn, empower Jace 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "PlanForAllOutcomes.PlanForAllOutcomesTriggeredAbility",
    "new SpellCastControllerTriggeredAbility(new EmpowerJaceEffect(1), StaticFilters.FILTER_SPELL_A_NON_CREATURE, false)",
    check,
)];

/// `SpellsCastWatcher`: it's the first noncreature spell you cast this turn
fn check(game: &Game, t: &StackItem) -> bool {
    game.player(t.controller).noncreature_cast == 1
}
