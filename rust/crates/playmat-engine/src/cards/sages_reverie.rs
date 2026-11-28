//! Sage's Reverie: "When this Aura enters, draw a card for each Aura you control that's attached to a creature.
//! Enchanted creature gets +1/+1 for each Aura you control that's attached to a creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("SagesReverie.SagesReveriePredicate", attached_to_creature)];

/// it's attached to a creature
fn attached_to_creature(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.attached(id).is_some_and(|to| game.has_type(to, CardType::Creature))
}
