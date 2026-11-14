//! Loot, the Nexus: "{T}: Choose a color. Add one mana of that color for each different power among creatures you
//! control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("LootTheNexus.LootTheNexusValue", different_powers)];

/// The different powers among the creatures you control
fn different_powers(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let mut powers: Vec<i32> = Vec::new();
    for o in game.battlefield.iter() {
        if game.obj(*o).controller == you && game.has_type(*o, CardType::Creature) {
            let p = game.power_toughness(*o).0;
            if !powers.contains(&p) {
                powers.push(p);
            }
        }
    }
    powers.len() as i32
}
