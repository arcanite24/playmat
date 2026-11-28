//! Liliana, the Last Hope: "−7: You get an emblem with "At the beginning of your end step, create X 2/2 black Zombie
//! creature tokens, where X is two plus the number of Zombies you control.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("LilianaTheLastHopeEmblem.LilianaZombiesCount", zombies_plus_two)];

/// the Zombies you control, plus two
fn zombies_plus_two(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    let zombies =
        game.battlefield.iter().filter(|o| game.obj(**o).controller == you && game.has_subtype(**o, "Zombie")).count();
    zombies as i32 + 2
}
