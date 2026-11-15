//! Overwrite the Multiverse: "Exile all creatures. Empower Jace X, where X is the number of creatures exiled this
//! way."

use super::*;
use crate::card::Amount;

pub const HANDLERS: &[Handler] = &[Handler::effect("OverwriteTheMultiverse.OverwriteTheMultiverseEffect", apply)
    .tokens(&["JacePlaneswalkerToken"])
    .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE"])];

/// every creature exiled together, then `EmpowerJaceEffect(n)`, n the creatures there were to exile
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = matching(game, &cx.filters[0], cx.controller(), cx.source_id()).len() as i32;
    cx.run(game, d, &[EffectDef::ExileAllEffect { filter: cx.filters[0].clone() }]);
    if d.stopped() {
        return None;
    }
    cx.run(game, d, &[EffectDef::EmpowerJaceEffect { amount: Amount::Fixed(n), token: cx.tokens[0].clone() }]);
    if d.stopped() {
        return None;
    }
    Some(())
}
