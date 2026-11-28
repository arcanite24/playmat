//! `VariableManaCostPredicate`: a card or spell with {X} in its mana cost (Frontline Medic, Matterbending Mage,
//! Paradox Surveyor, Geometer's Arthropod)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("VariableManaCostPredicate", has_x)];

/// `getManaCost()` has a `VariableManaCost`
fn has_x(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.obj(id).def.cost.x > 0
}
