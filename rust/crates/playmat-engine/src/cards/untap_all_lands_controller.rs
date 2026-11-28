//! `UntapAllLandsControllerEffect` (Bumi, Unleashed): "untap all lands you control"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UntapAllLandsControllerEffect", apply).args(&["filter"])];

/// Each land (of the filter, `StaticFilters.FILTER_LANDS` by default) you control untaps
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.last().cloned().unwrap_or(Filter(vec![Pred::Type { t: CardType::Land }]));
    let all: Vec<ObjId> = game.battlefield.iter().copied().collect();
    for o in all {
        if game.obj(o).controller == you && game.filter_matches(&filter, you, Some(cx.source_id()), o) {
            untap_permanent(game, o);
        }
    }
    Some(())
}
