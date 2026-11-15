//! Barrin, Tolarian Archmage: "When Barrin enters, return up to one other target creature or planeswalker to its
//! owner's hand. At the beginning of your end step, if a permanent was put into your hand from the battlefield this
//! turn, draw a card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("BarrinTolarianArchmage.BarrinTolarianArchmageCondition", returned_to_your_hand)];

/// `BarrinTolarianArchmageWatcher`: a permanent you own went from the battlefield to a hand this turn
fn returned_to_your_hand(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.left_battlefield.iter().any(|l| l.to == Zone::Hand && l.owner == you)
}
