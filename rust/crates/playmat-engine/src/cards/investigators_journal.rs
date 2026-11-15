//! Investigator's Journal: "This artifact enters with a number of suspect counters on it equal to the greatest number
//! of creatures a player controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("InvestigatorsJournal.InvestigatorsJournalValue", most_creatures)];

/// the most creatures any player in range controls
fn most_creatures(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    (0..game.seats())
        .filter(|p| !game.player(*p).lost)
        .map(|p| {
            game.battlefield
                .iter()
                .filter(|o| game.obj(**o).controller == p && game.has_type(**o, CardType::Creature))
                .count() as i32
        })
        .max()
        .unwrap_or(0)
}
