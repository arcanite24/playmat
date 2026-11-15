//! Squirming Emergence: "Fathomless descent — Return to the battlefield target nonland permanent card in your
//! graveyard with mana value less than or equal to the number of permanent cards in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("SquirmingEmergence.SquirmingEmergencePredicate", at_most_permanent_cards)];

/// Mana value at most the permanent cards in the source's controller's graveyard (you, choosing its targets)
fn at_most_permanent_cards(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let n = game
        .player(you)
        .graveyard
        .iter()
        .filter(|c| game.obj(**c).def.is_permanent_card())
        .count();
    game.obj(id).def.mana_value() as usize <= n
}
