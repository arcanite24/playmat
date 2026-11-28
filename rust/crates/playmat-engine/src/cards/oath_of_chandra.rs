//! Oath of Chandra: "When Oath of Chandra enters, it deals 3 damage to target creature an opponent controls. At the
//! beginning of each end step, if a planeswalker entered the battlefield under your control this turn, Oath of
//! Chandra deals 2 damage to each opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("OathOfChandra.OathOfChandraCondition", planeswalker_entered)];

/// `OathOfChandraWatcher`: a planeswalker entered under your control this turn
pub(super) fn planeswalker_entered(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.entered_permanents.iter().any(|(o, _, c)| *c == you && game.has_type(*o, CardType::Planeswalker))
}
