//! Fblthp, Knows the Way: "Domain — Fblthp's power is equal to the number of basic land types among lands you
//! control. When Fblthp enters, search your library for up to X basic land cards with different names, reveal
//! them, put them into your hand, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FblthpKnowsTheWay.FblthpKnowsTheWayEffect", apply).effects(&[
    "new SearchLibraryPutInHandEffect(new TargetCardWithDifferentNameInLibrary(0, 1, StaticFilters.FILTER_CARD_BASIC_LANDS), true)",
])];

/// `SearchLibraryPutInHandEffect` for up to X, the X paid for the permanent
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mut e = cx.effects.first()?.clone();
    if let EffectDef::SearchLibraryEffect { max, .. } = &mut e {
        *max = cx.item.x.max(0) as usize;
    }
    cx.run(game, d, &[e]);
    Some(())
}
