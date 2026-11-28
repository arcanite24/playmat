//! `YouLostLifeCondition` (Essence Channeler): "as long as you've lost life this turn"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("YouLostLifeCondition", holds)];

/// `PlayerLostLifeWatcher.getLifeLost(you) > 0`
fn holds(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).life_lost > 0
}
