//! Rhino, Barreling Brute: "Whenever Rhino attacks, if you've cast a spell with mana value 4 or greater this turn,
//! draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("RhinoBarrelingBrute.RhinoBarrelingBruteCondition", cast)];

/// `RhinoBarrelingBruteWatcher`: the players who cast a spell with mana value 4 or greater this turn
fn cast(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.spells.iter().any(|s| s.caster == you && s.mv >= 4)
}
