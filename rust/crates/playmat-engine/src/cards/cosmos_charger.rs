//! Cosmos Charger: "Foretelling cards from your hand costs {1} less and can be done on any player's turn."
//! `CosmosChargerCostReductionEffect` reduces a `ForetellAbility` of yours by {1} (`CardUtil.reduceCost`, generic
//! only); `CosmosChargerAllowForetellAnytime` is the `ALLOW_FORETELL_ANYTIME` as-though, whose `applies` is true for
//! everyone: `ForetellAbility.canActivate` only asks whether any such effect applies, so any player may foretell on
//! any turn while it's out.

use super::*;
use crate::card::{AbilityDef, ManaCost};

pub const HANDLERS: &[Handler] = &[
    Handler::cost("CosmosCharger.CosmosChargerCostReductionEffect", CostKind::Reduce, yours_foretell, one_less),
    Handler::as_though("CosmosCharger.CosmosChargerAllowForetellAnytime", AsThough::ForetellAnyTime(any_time)),
];

fn yours_foretell(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.seat == cx.controller && matches!(of.ability, Some(AbilityDef::ForetellAbility { .. }))
}

fn one_less(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(1);
}

fn any_time(_: &Game, _: &StaticCx, _: Seat) -> bool {
    true
}
