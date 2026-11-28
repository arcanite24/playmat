//! Invasion of Azgol // Ashen Reaper: "At the beginning of your end step, put a +1/+1 counter on Ashen Reaper if a
//! permanent was put into a graveyard from the battlefield this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("InvasionOfAzgol.AshenReaperCondition", a_permanent_died)];

/// `AshenReaperWatcher`: any permanent (a token too) went from the battlefield to a graveyard this turn
fn a_permanent_died(game: &Game, _: Seat, _: ObjId) -> bool {
    !game.watch.dies.is_empty()
}
