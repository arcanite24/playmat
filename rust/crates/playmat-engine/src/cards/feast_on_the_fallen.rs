//! Feast on the Fallen: "At the beginning of each upkeep, if an opponent lost life last turn, put a +1/+1 counter on
//! target creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("FeastOnTheFallen.FeastOnTheFallenCondition", opponent_lost_life)];

/// `PlayerLostLifeWatcher.getLifeLostLastTurn` of an opponent is above 0
fn opponent_lost_life(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.players.iter().enumerate().any(|(i, p)| i != you && p.life_lost_last_turn > 0)
}
