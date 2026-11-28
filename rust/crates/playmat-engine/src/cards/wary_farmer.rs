//! Wary Farmer: "At the beginning of your end step, if another creature entered the battlefield under your control
//! this turn, surveil 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("WaryFarmer.WaryFarmerCondition", another_entered)];

/// `WaryFarmerWatcher`: a creature entered under your control this turn that isn't the source as it is now
fn another_entered(game: &Game, you: Seat, source: ObjId) -> bool {
    let now = (source, game.obj(source).zcc);
    game.watch
        .entered
        .iter()
        .zip(game.watch.entered_permanents.iter())
        .any(|((who, _, creature, _), (id, zcc, _))| *who == you && *creature && (*id, *zcc) != now)
}
