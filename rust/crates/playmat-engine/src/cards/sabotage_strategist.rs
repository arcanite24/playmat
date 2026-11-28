//! Sabotage Strategist: "Flying, vigilance. Whenever one or more creatures attack you, those creatures get -1/-0
//! until end of turn. Exhaust — {5}{U}{U}: Put three +1/+1 counters on this creature."

use super::*;

/// (`DEFENDER_ATTACKED` at you, `FixedTargets` of that defender's attackers)
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SabotageStrategist.SabotageStrategistTriggeredAbility",
    "new AttacksYouTriggeredAbility(new BoostTargetEffect(-1, 0).setText(\"those creatures get -1/-0 until end of turn\"))",
    any_batch,
)];
