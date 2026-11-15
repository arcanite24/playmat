//! `VariableManaCostPredicate`: "with {X} in its mana cost" (Geometer's Arthropod)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("VariableManaCostPredicate", has_x)];

/// `getManaCost().containsX()`
fn has_x(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.obj(id).def.cost.x > 0
}
