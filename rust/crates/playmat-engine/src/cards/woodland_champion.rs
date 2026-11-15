//! Woodland Champion: "Whenever one or more tokens you control enter, put that many +1/+1 counters on this
//! creature."

use super::*;

/// (Java adds the counters effect as it triggers, with the batch's count: the trigger's X)
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "WoodlandChampion.WoodlandChampionTriggeredAbility",
    "new ZoneChangeBatchTriggeredAbility(new AddCountersSourceEffect(CounterType.P1P1.createInstance(), GetXValue.instance), null, Zone.BATTLEFIELD, StaticFilters.FILTER_PERMANENT_TOKEN, TargetController.YOU, false)",
    any_batch,
)];
