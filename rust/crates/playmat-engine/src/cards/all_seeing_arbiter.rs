//! All-Seeing Arbiter: "Whenever you discard a card, target creature an opponent controls gets -X/-0 until your next
//! turn, where X is the number of different mana values among cards in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("AllSeeingArbiter.AllSeeingArbiterValue", minus_mana_values)];

/// minus the number of different mana values in your graveyard
fn minus_mana_values(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    let mut values: Vec<u32> = game.player(you).graveyard.iter().map(|c| game.obj(*c).def.mana_value()).collect();
    values.sort_unstable();
    values.dedup();
    -(values.len() as i32)
}
