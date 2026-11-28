//! Oblivion's Hunger: "Target creature you control gains indestructible until end of turn. Draw a card if that
//! creature has a +1/+1 counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("OblivionsHunger.OblivionsHungerCondition", has_counter)];

/// the first target (`getFirstTarget`), as a permanent, has a +1/+1 counter
fn has_counter(game: &Game, _: Seat, _: ObjId) -> bool {
    let Some(item) = game.resolving_item.as_ref() else { return false };
    let Some(Target::Obj(o)) = item.targets.first().and_then(|ts| ts.first()).copied() else { return false };
    game.obj(o).zone == Zone::Battlefield && game.obj(o).counter("+1/+1") > 0
}
