//! Sidequest: Hunt the Mark: "At the beginning of your end step, if a creature died under an opponent's control this
//! turn, create a Treasure token. Then if you control three or more Treasures, transform this enchantment."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("SidequestHuntTheMark.SidequestHuntTheMarkCondition", opponent_creature_died)];

/// its watcher adds the opponents of each dying creature's controller: a creature died this turn under the control
/// of a player other than you
fn opponent_creature_died(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.watch.died.iter().any(|c| *c != you)
}
