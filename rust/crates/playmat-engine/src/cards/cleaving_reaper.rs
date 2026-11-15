//! Cleaving Reaper: "Pay 3 life: Return Cleaving Reaper from your graveyard to your hand. Activate only if you had an
//! Angel or Berserker enter the battlefield under your control this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("CleavingReaper.CleavingReaperCondition", angel_or_berserker)];

/// `PermanentsEnteredBattlefieldWatcher.getThisTurnEnteringPermanents(you)`: one of them an Angel or a Berserker
fn angel_or_berserker(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch
        .entered_permanents
        .iter()
        .any(|(o, _, c)| *c == you && (game.has_subtype(*o, "Angel") || game.has_subtype(*o, "Berserker")))
}
