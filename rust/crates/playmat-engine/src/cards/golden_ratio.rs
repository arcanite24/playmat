//! Golden Ratio: "Draw a card for each different power among creatures you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("GoldenRatio.GoldenRatioValue", different_powers)];

/// the distinct powers among the creatures you control
fn different_powers(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    let mut seen: Vec<i32> = Vec::new();
    for c in game.battlefield.iter() {
        if game.obj(*c).controller == you && game.has_type(*c, CardType::Creature) {
            let p = game.power_toughness(*c).0;
            if !seen.contains(&p) {
                seen.push(p);
            }
        }
    }
    seen.len() as i32
}
