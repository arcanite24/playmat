//! `UntapAllControllerEffect` (Combat Celebrant): "untap all [other] creatures you control"

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("UntapAllControllerEffect", apply).args(&["filter", "text", "text"])];

/// Each permanent of the filter you control untaps (`Permanent.untap`), the source too unless `includeSource` is
/// false
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let include_source = cx.args.get(2).is_none_or(|a| a.trim() != "false");
    let filter = cx.filters.last().cloned().unwrap_or_default();
    let all: Vec<ObjId> = game.battlefield.iter().copied().collect();
    for o in all {
        if game.obj(o).controller != you || !game.filter_matches(&filter, you, Some(cx.source_id()), o) {
            continue;
        }
        if include_source || o != cx.source_id() {
            untap_permanent(game, o);
        }
    }
    Some(())
}
