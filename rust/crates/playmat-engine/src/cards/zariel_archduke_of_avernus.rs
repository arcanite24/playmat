//! Zariel, Archduke of Avernus's emblem: "At the end of the first combat phase on your turn, untap target creature
//! you control. After this phase, there is an additional combat phase."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ZarielArchdukeOfAvernusEmblem.ZarielArchdukeOfAvernusEmblemAbility",
    "new EndOfCombatTriggeredAbility(new UntapTargetEffect(), false)",
    first_combat_on_your_turn,
)];

/// `END_COMBAT_STEP_PRE` on your turn, of the turn's first combat phase
fn first_combat_on_your_turn(game: &Game, t: &StackItem) -> bool {
    game.active == Some(t.controller) && !game.combat_phase_done
}
