//! Rampaging Soulrager: "This creature gets +3/+0 as long as there are two or more unlocked doors among Rooms you
//! control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("RampagingSoulrager.RampagingSoulragerCondition", two_doors)];

/// the unlocked doors of the Rooms you control are two or more
fn two_doors(game: &Game, you: Seat, _: ObjId) -> bool {
    let doors: usize = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_subtype(**o, "Room"))
        .map(|o| game.obj(*o).doors.iter().filter(|d| **d).count())
        .sum();
    doors >= 2
}
