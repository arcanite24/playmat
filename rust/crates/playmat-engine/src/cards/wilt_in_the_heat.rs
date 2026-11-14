//! Wilt in the Heat: "This spell costs {2} less to cast if one or more cards left your graveyard this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("WiltInTheHeat.WiltInTheHeatCondition", left_your_graveyard)];

/// `CardsLeftGraveyardWatcher`: a card you own left your graveyard this turn
fn left_your_graveyard(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.watch.left_graveyard.contains(&you)
}
