//! Lilypad Village: "{T}: Add {C}. {T}: Add {U}. Spend this mana only to cast a creature spell. {U}, {T}: Surveil 2.
//! Activate only if a Bird, Frog, Otter, or Rat entered the battlefield under your control this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("LilypadVillage.LilypadVillageCondition", one_entered)];

/// `LilypadVillageWatcher`: a Bird, Frog, Otter or Rat entered under your control this turn (its types now)
fn one_entered(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.entered_permanents.iter().any(|(o, _, c)| {
        *c == you && ["Bird", "Frog", "Otter", "Rat"].iter().any(|t| game.has_subtype(*o, t))
    })
}
