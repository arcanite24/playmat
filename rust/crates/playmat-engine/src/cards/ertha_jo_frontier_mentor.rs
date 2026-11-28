//! Ertha Jo, Frontier Mentor: "Whenever you activate an ability that targets a creature or player, copy that
//! ability. You may choose new targets for the copy."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ErthaJoFrontierMentor.ErthaJoFrontierMentorTriggeredAbility",
    "new ActivateAbilityTriggeredAbility(new CopyStackObjectEffect(), filter, SetTargetPointer.SPELL)",
    any,
)];

/// `checkTrigger`: the filter (an ability that targets a creature or player) is the trigger's own kind
fn any(_: &Game, _: &StackItem) -> bool {
    true
}
