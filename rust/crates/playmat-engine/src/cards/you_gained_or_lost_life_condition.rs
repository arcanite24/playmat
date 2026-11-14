//! `YouGainedOrLostLifeCondition` (Starlit Soothsayer): "if you gained or lost life this turn".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("YouGainedOrLostLifeCondition", gained_or_lost)];

/// `PlayerGainedLifeWatcher` or `PlayerLostLifeWatcher` saw some this turn
fn gained_or_lost(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).life_gained > 0 || game.player(you).life_lost > 0
}
