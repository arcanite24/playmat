//! Alive // Well: "Create a 3/3 green Centaur creature token. // You gain 2 life for each creature you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("AliveWell.WellEffect", apply).filters(&["StaticFilters.FILTER_CONTROLLED_CREATURE"])];

/// `gainLife(2 * count)` (gaining 0 does nothing)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = matching(game, &cx.filters[0], you, cx.source_id()).len() as i32;
    gain_life(game, you, 2 * n);
    Some(())
}
