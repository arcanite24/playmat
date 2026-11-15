//! Bristlebane Outrider: "This creature can't be blocked by creatures with power 2 or less. As long as another
//! creature entered the battlefield under your control this turn, this creature gets +2/+0."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("BristlebaneOutrider.BristlebaneOutriderCondition", another_entered)];

/// `BristlebaneOutriderWatcher`: a creature that entered under your control this turn that isn't this permanent
/// (as the object it is now)
fn another_entered(game: &Game, you: Seat, source: ObjId) -> bool {
    let me = (source, game.obj(source).zcc);
    game.watch.entered.iter().zip(game.watch.entered_permanents.iter()).any(|(seen, (id, zcc, controller))| {
        seen.2 && *controller == you && (*id, *zcc) != me
    })
}
