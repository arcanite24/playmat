//! Lifecraft Engine: "As this Vehicle enters, choose a creature type. Vehicle creatures you control are the chosen
//! creature type in addition to their other types. Each creature you control of the chosen type other than this
//! Vehicle gets +1/+1. Crew 3"

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::continuous(
    "LifecraftEngine.LifecraftEngineAddSubTypeAllEffect",
    vehicle_creature_of_yours,
    Layer::Type(chosen_type),
)];

/// a Vehicle creature you control (`FilterControlledCreaturePermanent` with `SubType.VEHICLE`)
fn vehicle_creature_of_yours(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    let o = game.obj(id);
    o.zone == Zone::Battlefield
        && o.controller == cx.controller
        && game.has_type(id, CardType::Creature)
        && game.has_subtype(id, "Vehicle")
}

/// the creature type chosen as it entered (none chosen: nothing)
fn chosen_type(game: &Game, cx: &StaticCx, _: ObjId) -> Vec<String> {
    game.obj(cx.source).chosen_type.iter().cloned().collect()
}
