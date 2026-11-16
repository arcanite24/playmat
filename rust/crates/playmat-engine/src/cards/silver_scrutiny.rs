//! Silver Scrutiny: "You may cast Silver Scrutiny as though it had flash if X is 3 or less." The condition reads
//! the spell on the stack (`getStack().getSpell(source.getSourceId())`): its X is below 4; false with no spell.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("SilverScrutiny.SilverScrutinyCondition", x_at_most_three)];

fn x_at_most_three(game: &Game, _you: Seat, source: ObjId) -> bool {
    game.stack.iter().find(|i| i.card == source && i.ability.is_none()).is_some_and(|i| i.x < 4)
}
