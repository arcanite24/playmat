//! Mavren Fein, Dusk Apostle: "Whenever one or more nontoken Vampires you control attack, create a 1/1 white
//! Vampire creature token with lifelink."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "MavrenFeinDuskApostle.MavrenFeinDuskApostleTriggeredAbility",
    "new AttacksWithCreaturesTriggeredAbility(new CreateTokenEffect(new IxalanVampireToken()), 1, filter)",
    check,
)];

/// `checkTrigger` is the base trigger's own: one attacker of the filter (nontoken Vampires you control)
fn check(_game: &Game, _t: &StackItem) -> bool {
    true
}
