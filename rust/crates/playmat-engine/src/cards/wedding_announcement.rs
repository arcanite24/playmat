//! Wedding Announcement: "At the beginning of your end step, put an invitation counter on this enchantment. If you
//! attacked with two or more creatures this turn, draw a card. Otherwise, create a 1/1 white Human creature token.
//! Then if it has three or more invitation counters on it, transform it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("WeddingAnnouncement.WeddingAnnouncementCondition", two_attacked)];

/// `AttackedThisTurnWatcher.getAttackedThisTurnCreatures().size() >= 2`
fn two_attacked(game: &Game, _: Seat, _: ObjId) -> bool {
    game.attacked_this_turn.len() >= 2
}
