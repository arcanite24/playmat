//! `ControlsCreatureGreatestToughnessCondition` (Abzan Beastmaster): "if you control the creature with the greatest
//! toughness or tied for the greatest toughness".

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("ControlsCreatureGreatestToughnessCondition", controls_greatest_toughness)];

/// Among every creature on the battlefield, one of the greatest toughness is yours
fn controls_greatest_toughness(game: &Game, you: Seat, _source: ObjId) -> bool {
    let creatures: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| game.has_type(*o, CardType::Creature)).collect();
    let Some(max) = creatures.iter().map(|o| game.power_toughness(*o).1).max() else { return false };
    creatures.iter().any(|o| game.obj(*o).controller == you && game.power_toughness(*o).1 == max)
}
