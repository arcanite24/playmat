//! Might Makes Right: "At the beginning of combat on your turn, if you control each creature on the battlefield with
//! the greatest power, gain control of target creature an opponent controls until end of turn. Untap that creature.
//! It gains haste until end of turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("MightMakesRight.ControlsEachCreatureWithGreatestPowerCondition", greatest_power)];

/// Some creature is on the battlefield, and each one with the greatest power is yours
fn greatest_power(game: &Game, you: Seat, _: ObjId) -> bool {
    let creatures: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| game.has_type(*o, CardType::Creature)).collect();
    let Some(max) = creatures.iter().map(|o| game.power_toughness(*o).0).max() else { return false };
    creatures.iter().filter(|o| game.power_toughness(**o).0 >= max).all(|o| game.obj(*o).controller == you)
}
