//! Eshki Dragonclaw: "At the beginning of combat on your turn, if you've cast both a creature spell and a
//! noncreature spell this turn, draw a card and put two +1/+1 counters on Eshki Dragonclaw."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("EshkiDragonclaw.EshkiDragonclawCondition", both_cast)];

/// `EshkiDragonclawWatcher`: a creature spell and a noncreature spell cast by you this turn
fn both_cast(game: &Game, you: Seat, _: ObjId) -> bool {
    let mine = || game.watch.spells.iter().filter(|s| s.caster == you);
    mine().any(|s| s.creature) && mine().any(|s| !s.creature)
}
