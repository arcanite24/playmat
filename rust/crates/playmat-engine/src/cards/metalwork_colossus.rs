//! Metalwork Colossus: "This spell costs {X} less to cast, where X is the total mana value of noncreature
//! artifacts you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("MetalworkColossus.TotalPermanentsManaValue", total_mana_value)];

/// `TotalPermanentsManaValue(filterCostReduction)`: noncreature artifacts you control
fn total_mana_value(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.battlefield
        .iter()
        .filter(|o| {
            game.obj(**o).controller == you
                && game.has_type(**o, CardType::Artifact)
                && !game.has_type(**o, CardType::Creature)
        })
        .map(|o| game.obj(*o).def.mana_value() as i32)
        .sum()
}
