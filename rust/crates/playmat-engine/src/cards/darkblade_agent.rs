//! Darkblade Agent: "As long as you've surveilled this turn, this creature has deathtouch and "Whenever this
//! creature deals combat damage to a player, draw a card.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("DarkbladeAgent.DarkbladeAgentCondition", surveilled)];

/// `DarkbladeAgentWatcher`: `SURVEILED` of yours this turn
fn surveilled(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.surveiled.contains(&you)
}
