//! Cult of the Waxing Moon: "Whenever a permanent you control transforms into a non-Human creature, create a 2/2
//! green Wolf creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "CultOfTheWaxingMoon.CultOfTheWaxingMoonAbility",
    "new TransformsAllTriggeredAbility(new CreateTokenEffect(new WolfToken()), filter)",
    any_batch,
)];
