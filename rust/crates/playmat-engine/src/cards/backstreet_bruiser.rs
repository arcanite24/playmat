//! Backstreet Bruiser: "Defender. As long as there are two or more counters among creatures you control, Backstreet
//! Bruiser can attack as though it didn't have defender."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("BackstreetBruiser.BackstreetBruiserCondition", two_counters)];

/// Every kind of counter on the creatures you control, counted together
fn two_counters(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .flat_map(|o| game.obj(*o).counters.iter())
        .map(|(_, n)| *n)
        .sum::<i32>()
        >= 2
}
