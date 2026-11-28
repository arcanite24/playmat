//! Selvala, Eager Trailblazer: "Vigilance. Whenever you cast a creature spell, create a 1/1 red Mercenary creature
//! token with "{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery." {T}:
//! Choose a color. Add one mana of that color for each different power among creatures you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("SelvalaEagerTrailblazer.SelvalaEagerTrailblazerValue", different_powers)];

/// the number of different powers among creatures you control
fn different_powers(game: &Game, you: Seat, _: ObjId, _x: i32) -> i32 {
    let mut powers: Vec<i32> = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .map(|o| game.power_toughness(*o).0)
        .collect();
    powers.sort_unstable();
    powers.dedup();
    powers.len() as i32
}
