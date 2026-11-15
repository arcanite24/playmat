//! Dwindle: "Enchant creature. Enchanted creature gets -6/-0. When enchanted creature blocks, destroy it."

use super::*;

/// `BlocksAttachedTriggeredAbility`: a creature blocking (the pointer) that this Aura is attached to; its
/// `DestroyAttachedToEffect` destroys that creature
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "Dwindle.DwindleTriggeredAbility",
    "new BlocksAllTriggeredAbility(new DestroyTargetEffect(), false, StaticFilters.FILTER_PERMANENT_CREATURE, SetTargetPointer.PERMANENT)",
    check,
)];

/// the blocker is the creature this Aura enchants (or enchanted as it left the battlefield)
fn check(game: &Game, t: &StackItem) -> bool {
    let Some(Target::Obj(blocker)) = t.targets.first().and_then(|g| g.first()) else { return false };
    let aura = game.obj(t.card);
    let on = if aura.zone == Zone::Battlefield { aura.attached_to } else { aura.lki_attached_to };
    on.is_some_and(|(o, _)| o == *blocker)
}
