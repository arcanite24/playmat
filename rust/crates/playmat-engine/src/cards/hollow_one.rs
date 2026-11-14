//! Hollow One: "This spell costs {2} less to cast for each card you've cycled or discarded this turn."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("HollowOne.HollowOneReductionEffect", CostKind::Reduce, applies, reduce).in_any_zone()];

/// its own spell
fn applies(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.card == cx.source
}

/// `CardsCycledOrDiscardedThisTurnWatcher`: a cycled card is discarded, the same object, so each card counts once
fn reduce(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let n = game.watch.discarded.iter().filter(|s| **s == cx.controller).count() as u32;
    cost.generic = cost.generic.saturating_sub(2 * n);
}
