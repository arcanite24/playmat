//! Sudden Insight: "Draw a card for each different mana value among nonland cards in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("SuddenInsight.SuddenInsightValue", values)];

fn values(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let mut seen: Vec<u32> = Vec::new();
    for c in &game.player(you).graveyard {
        let mv = game.obj(*c).def.mana_value();
        if !game.has_type(*c, CardType::Land) && !seen.contains(&mv) {
            seen.push(mv);
        }
    }
    seen.len() as i32
}
