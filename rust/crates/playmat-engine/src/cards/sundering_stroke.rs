//! Sundering Stroke: "… If at least seven red mana was spent to cast this spell, instead Sundering Stroke deals 7
//! damage to each of those permanents and/or players."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("SunderingStroke.SunderingStrokeCondtition", red_spent)];

/// Java's check is `getUsedManaToPay().getColor(R) >= 6` (six, not seven, as written): the red spent to cast it
fn red_spent(game: &Game, you: Seat, source: ObjId) -> bool {
    let c = crate::card::Condition::ManaColorSpent { color: crate::card::Color::R, n: 6 };
    game.condition_holds(Some(&c), you, source)
}
