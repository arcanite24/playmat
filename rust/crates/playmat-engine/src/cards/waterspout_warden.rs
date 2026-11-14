//! Waterspout Warden: "Whenever Waterspout Warden attacks, if another creature entered the battlefield under your
//! control this turn, Waterspout Warden gains flying until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("WaterspoutWarden.WaterspoutWardenCondition", another)];

/// `WaterspoutWardenWatcher`: a creature that entered under your control this turn, other than this permanent
fn another(game: &Game, you: Seat, source: ObjId) -> bool {
    let zcc = game.obj(source).zcc;
    game.watch
        .entered
        .iter()
        .zip(game.watch.entered_permanents.iter())
        .any(|((_, _, creature, _), (o, z, seat))| *creature && *seat == you && !(*o == source && *z == zcc))
}
