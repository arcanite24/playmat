//! Vizkopa Guildmage: "{1}{W}{B}: Target creature gains lifelink until end of turn. {1}{W}{B}: Whenever you gain
//! life this turn, each opponent loses that much life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "VizkopaGuildmage.VizkopaGuildmageDelayedTriggeredAbility",
    "new GainLifeControllerTriggeredAbility(new LoseLifeOpponentsEffect(SavedGainedLifeValue.MUCH))",
    you_gain_life,
)];

/// you gaining life (`GAINED_LIFE` for you), its amount the X: the base trigger is all of it
fn you_gain_life(_: &Game, _: &StackItem) -> bool {
    true
}
