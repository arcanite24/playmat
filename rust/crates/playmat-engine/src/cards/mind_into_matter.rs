//! Mind into Matter: "Draw X cards. Then you may put a permanent card with mana value X or less from your hand onto
//! the battlefield tapped."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("MindIntoMatter.MindIntoMatterPredicate", at_most_x)];

/// Its mana value is at most the spell's X (`GetXValue`): the resolving spell's, or the one on the stack
fn at_most_x(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let x = game
        .resolving_item
        .as_ref()
        .filter(|i| i.card == src && i.ability.is_none())
        .or_else(|| game.stack.iter().rev().find(|i| i.card == src && i.ability.is_none()))
        .map_or(0, |i| i.x);
    game.obj(id).def.mana_value() as i32 <= x
}
