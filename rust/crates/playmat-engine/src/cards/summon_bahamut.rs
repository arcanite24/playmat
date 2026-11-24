//! Summon: Bahamut: "IV — Mega Flare — This creature deals damage equal to the total mana value of other permanents
//! you control to each opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("SummonBahamut.SummonBahamutValue", total_mana_value)];

/// The mana values of the other permanents you control, summed
fn total_mana_value(game: &Game, you: Seat, source: ObjId, _: i32) -> i32 {
    game.battlefield
        .iter()
        .filter(|o| **o != source && game.obj(**o).controller == you)
        .map(|o| game.obj(*o).def.mana_value() as i32)
        .sum()
}
