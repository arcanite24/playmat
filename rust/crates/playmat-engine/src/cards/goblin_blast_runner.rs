//! Goblin Blast-Runner: "Goblin Blast-Runner gets +2/+0 and has menace as long as you sacrificed a permanent this
//! turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("GoblinBlastRunner.GoblinBlastRunnerCondition", sacrificed)];

/// `PermanentsSacrificedWatcher.getThisTurnSacrificedPermanents(you)`
fn sacrificed(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.watch.sacrificed.iter().any(|(s, _)| *s == you)
}
