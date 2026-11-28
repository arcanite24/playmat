//! Knight of the Ebon Legion: "At the beginning of your end step, if a player lost 4 or more life this turn, put a
//! +1/+1 counter on Knight of the Ebon Legion."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("KnightOfTheEbonLegion.KnightOfTheEbonLegionCondition", a_player_lost_four)];

/// `PlayerLostLifeWatcher.getLifeLost` above 3 for a player in range
fn a_player_lost_four(game: &Game, _you: Seat, _source: ObjId) -> bool {
    (0..game.seats()).any(|s| !game.player(s).lost && game.player(s).life_lost > 3)
}
