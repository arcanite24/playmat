//! Rakshasa Vizier: "Whenever one or more cards are put into exile from your graveyard, put that many +1/+1
//! counters on this creature."

use super::*;

/// (Java adds the counters effect as it triggers, with the batch's count: the trigger's X)
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "RakshasaVizier.RakshasaVizierTriggeredAbility",
    "new ZoneChangeBatchTriggeredAbility(new AddCountersSourceEffect(CounterType.P1P1.createInstance(), GetXValue.instance), Zone.GRAVEYARD, Zone.EXILED, StaticFilters.FILTER_CARD, TargetController.YOU, true)",
    any_batch,
)];
