//! `ControlsCreatureGreatestPowerCondition` (Summon: Fenrir): "if you control the creature with the greatest power or
//! tied for the greatest power".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("ControlsCreatureGreatestPowerCondition", greatest)];

/// One of the creatures on the battlefield with the greatest power is yours (no creature: false)
fn greatest(game: &Game, you: Seat, _: ObjId) -> bool {
    let creatures: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| game.has_type(*o, CardType::Creature)).collect();
    let Some(most) = creatures.iter().map(|o| game.power_toughness(*o).0).max() else { return false };
    creatures.iter().any(|o| game.obj(*o).controller == you && game.power_toughness(*o).0 == most)
}
