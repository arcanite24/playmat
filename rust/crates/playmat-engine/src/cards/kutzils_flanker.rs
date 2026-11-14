//! Kutzil's Flanker: "Flash. When this creature enters, choose one — • Put a +1/+1 counter on this creature for each
//! creature that left the battlefield under your control this turn. • You gain 2 life and scry 2. • Exile target
//! player's graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("KutzilsFlanker.KutzilsFlankerValue", creatures_left)];

/// `CreatureLeftBattlefieldWatcher.getNumberCreatureLeft`: the creatures that left the battlefield under your control
fn creatures_left(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.watch.left_battlefield.iter().filter(|l| l.creature && l.controller == you).count() as i32
}
