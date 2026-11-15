//! Bonecache Overseer: "{T}, Pay 1 life: Draw a card. Activate only if three or more cards left your graveyard this
//! turn or if you've sacrificed a Food this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("BonecacheOverseer.BonecacheOverseerCondition", condition)];

/// `BonecacheOverseerWatcher`: three cards you own left a graveyard this turn, or you sacrificed a Food
fn condition(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.left_graveyard.iter().filter(|p| **p == you).count() >= 3
        || game.watch.sacrificed_objs.iter().any(|(s, o, _)| *s == you && game.obj(*o).def.has_subtype("Food"))
}
