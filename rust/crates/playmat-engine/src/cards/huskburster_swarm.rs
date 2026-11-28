//! Huskburster Swarm: "This spell costs {1} less to cast for each creature card you own in exile and in your
//! graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("HuskbursterSwarm.HuskbursterSwarmValue", creature_cards)];

/// creature cards you own in exile, plus creature cards in your graveyard
fn creature_cards(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    let exiled = game
        .exile
        .iter()
        .filter(|o| game.obj(**o).owner == you && game.has_type(**o, CardType::Creature))
        .count();
    let buried = game.player(you).graveyard.iter().filter(|o| game.has_type(**o, CardType::Creature)).count();
    (exiled + buried) as i32
}
