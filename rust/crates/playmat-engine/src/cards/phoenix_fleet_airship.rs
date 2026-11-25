//! Phoenix Fleet Airship: "At the beginning of your end step, if you sacrificed a permanent this turn, create a token
//! that's a copy of this Vehicle."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("PhoenixFleetAirship.PhoenixFleetAirshipCondition", sacrificed)];

/// `PermanentsSacrificedThisTurnCount.YOU > 0` (`PermanentsSacrificedWatcher`)
fn sacrificed(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.watch.sacrificed.iter().any(|(s, _)| *s == you)
}
