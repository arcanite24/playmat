//! Cosmos Elixir: "At the beginning of your end step, draw a card if your life total is greater than your starting
//! life total. Otherwise, you gain 2 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("CosmosElixir.CosmosElixirCondition", above_starting_life)];

fn above_starting_life(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).life > game.starting_life
}
