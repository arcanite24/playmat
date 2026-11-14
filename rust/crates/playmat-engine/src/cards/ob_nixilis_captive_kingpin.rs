//! Ob Nixilis, Captive Kingpin: "Whenever one or more opponents each lose exactly 1 life, put a +1/+1 counter on Ob
//! Nixilis. Exile the top card of your library. Until your next end step, you may play that card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ObNixilisCaptiveKingpin.ObNixilisCaptiveKingpinAbility",
    "new LoseLifeTriggeredAbility($0, TargetController.OPPONENT, false, false)",
    exactly_one,
)];

/// `LOST_LIFE_BATCH`: the opponent (a duel has one) lost exactly 1 life in the batch, its X
fn exactly_one(_: &Game, t: &StackItem) -> bool {
    t.x == 1
}
