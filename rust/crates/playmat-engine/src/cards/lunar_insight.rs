//! Lunar Insight: "Draw a card for each different mana value among nonland permanents you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("LunarInsight.LunarInsightValue", value)];

/// the distinct mana values of the nonland permanents you control
fn value(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let mut seen: Vec<u32> = Vec::new();
    for o in game.battlefield.iter().copied() {
        if game.obj(o).controller == you && !game.has_type(o, CardType::Land) {
            let mv = game.obj(o).def.mana_value();
            if !seen.contains(&mv) {
                seen.push(mv);
            }
        }
    }
    seen.len() as i32
}
