//! Sporogenic Infection: "Enchant creature. When Sporogenic Infection enters, target player sacrifices a creature of
//! their choice other than enchanted creature. When enchanted creature is dealt damage, destroy it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("SporogenicInfection.SporogenicInfectionPredicate", not_enchanted)];

/// not the creature the Aura (as it last was) is attached to
fn not_enchanted(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    !source.and_then(|s| game.obj(s).attached_to).is_some_and(|(to, _)| to == id)
}
