//! Steel Exemplar: "Steel Exemplar enters with two +1/+1 counters on it unless two or more colors of mana were spent
//! to cast it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("SteelExemplar.SteelExemplarCondition", fewer_than_two)];

/// `ColorsOfManaSpentToCastCount < 2`
fn fewer_than_two(game: &Game, you: Seat, source: ObjId) -> bool {
    let Ok(colors) = serde_json::from_value::<Amount>(serde_json::json!({"v": "colors_spent"})) else { return false };
    game.value(&colors, you, source, 0) < 2
}
