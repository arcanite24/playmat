//! Gleam of Authority: "Enchanted creature gets +1/+1 for each +1/+1 counter on other creatures you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("GleamOfAuthority.CountersOnControlledCount", counters)];

/// the +1/+1 counters on the creatures you control other than the one this Aura is attached to
fn counters(game: &Game, you: Seat, source: ObjId, _: i32) -> i32 {
    let enchanted = game.attached(source);
    game.battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature) && Some(**o) != enchanted)
        .map(|o| game.obj(*o).counter("+1/+1"))
        .sum()
}
