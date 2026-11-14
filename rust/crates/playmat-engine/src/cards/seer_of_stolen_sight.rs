//! Seer of Stolen Sight: "Whenever one or more artifacts and/or creatures you control are put into a graveyard from
//! the battlefield, surveil 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SeerOfStolenSight.SeerOfStolenSightTriggeredAbility",
    "new ZoneChangeBatchTriggeredAbility(new SurveilEffect(1), Zone.BATTLEFIELD, Zone.GRAVEYARD, StaticFilters.FILTER_PERMANENT_ARTIFACT_OR_CREATURE, TargetController.YOU, false)",
    any_batch,
)];
