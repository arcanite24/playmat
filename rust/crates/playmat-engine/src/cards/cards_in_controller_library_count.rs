//! `CardsInControllerLibraryCount`: "the number of cards in your library" (Body of Research, Enter the Infinite)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("CardsInControllerLibraryCount", library_size)];

fn library_size(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.player(you).library.len() as i32
}
