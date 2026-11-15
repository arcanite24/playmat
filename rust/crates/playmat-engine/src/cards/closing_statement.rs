//! Closing Statement: "This spell costs {2} less to cast during your end step. Destroy target creature or planeswalker
//! you don't control. Put a +1/+1 counter on up to one target creature you control."

use super::*;
use crate::state::Step;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("ClosingStatement.IsPhaseCondition", your_end_phase),
    Handler::effect("ClosingStatement.ClosingStatementEffect", counter),
];

/// `IsPhaseCondition(TurnPhase.END, true)`: the end phase (its end step or cleanup) of your turn
fn your_end_phase(game: &Game, you: Seat, _: ObjId) -> bool {
    matches!(game.step, Some(Step::EndTurn | Step::Cleanup)) && game.active == Some(you)
}

/// a +1/+1 counter on the second target (tag 2), while it's on the battlefield
fn counter(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if let Some(o) = cx.obj(1).filter(|o| game.obj(*o).zone == Zone::Battlefield) {
        add_counters_by(game, o, "+1/+1", 1, Some(cx.controller()));
    }
    Some(())
}
