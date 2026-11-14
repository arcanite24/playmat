//! Desolation Twin: "When you cast this spell, create a 10/10 colorless Eldrazi creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "DesolationTwin.DesolationTwinOnCastAbility",
    "new CastSourceTriggeredAbility(new CreateTokenEffect(new EldraziToken()))",
    any,
)];

fn any(_: &Game, _: &StackItem) -> bool {
    true
}
