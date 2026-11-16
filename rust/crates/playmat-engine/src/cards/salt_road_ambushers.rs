//! Salt Road Ambushers: "Whenever another permanent you control is turned face up, if it's a creature, put two +1/+1
//! counters on it. Megamorph {3}{G}{G}"

use super::*;

/// (Java's class checks no more than its base: the permanent turned face up, of the filter, is the pointer)
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SaltRoadAmbushers.SaltRoadAmbushersTriggeredAbility",
    "new TurnedFaceUpAllTriggeredAbility(new AddCountersTargetEffect(CounterType.P1P1.createInstance(2)), filter, true)",
    any_batch,
)];
