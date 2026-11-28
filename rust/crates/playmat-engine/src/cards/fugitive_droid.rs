//! Fugitive Droid: "This creature can't be blocked if an artifact entered the battlefield under your control this
//! turn. {U}, Sacrifice this creature: Counter target spell that targets an artifact or creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("FugitiveDroid.FugitiveDroidCondition", artifact_entered)];

/// `FugitiveDroidWatcher`: an artifact entered under your control this turn
fn artifact_entered(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.entered.iter().any(|(c, artifact, _, _)| *c == you && *artifact)
}
