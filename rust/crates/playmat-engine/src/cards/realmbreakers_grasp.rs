//! Realmbreaker's Grasp: "Enchanted permanent can't attack or block, and its activated abilities can't be activated
//! unless they're mana abilities."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("RealmbreakersGrasp.RealmbreakersGraspEffect", Rule::CantActivate(enchanted))];

/// an ability (not a mana ability) of the permanent the Aura is attached to
fn enchanted(game: &Game, cx: &StaticCx, _: Seat, id: ObjId, _: usize) -> bool {
    game.obj(cx.source).zone == Zone::Battlefield && game.attached(cx.source) == Some(id)
}
