//! Path of Bravery: "As long as your life total is greater than or equal to your starting life total, creatures you
//! control get +1/+1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("PathOfBravery.PathOfBraveryCondition", at_starting_life)];

/// your life is at least the game's starting life
fn at_starting_life(game: &Game, you: Seat, _: ObjId) -> bool {
    game.player(you).life >= game.starting_life
}
