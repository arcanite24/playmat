//! Dinosaur Hunter: "Whenever Dinosaur Hunter deals damage to a Dinosaur, destroy that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "DinosaurHunter.DinosaurHunterAbility",
    "new DealsDamageToACreatureTriggeredAbility(new DestroyTargetEffect(), false, false, true)",
    to_a_dinosaur,
)];

/// the permanent it dealt damage to (as it last was) is a Dinosaur
fn to_a_dinosaur(game: &Game, item: &StackItem) -> bool {
    matches!(item.targets.first().and_then(|t| t.first()), Some(Target::Obj(o)) if game.has_subtype(*o, "Dinosaur"))
}
