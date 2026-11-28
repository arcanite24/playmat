//! Darklight Phoenix: "Flying, haste. At the beginning of combat on your turn, if two or more creatures died this
//! turn, you may pay 2 life. If you do, return this card from your graveyard to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("DarklightPhoenix.DarklightPhoenixCondition", two_died)];

/// `CreaturesDiedWatcher.getAmountOfCreaturesDiedThisTurn`: two or more, any player's
fn two_died(game: &Game, _: Seat, _: ObjId) -> bool {
    game.watch.died.len() >= 2
}
