//! Old-Growth Dryads: "When Old-Growth Dryads enters, each opponent may search their library for a basic land card,
//! put it onto the battlefield tapped, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OldGrowthDryads.OldGrowthDryadsEffect", apply).effects(&[
    "new SearchLibraryPutInPlayEffect(new TargetCardInLibrary(StaticFilters.FILTER_CARD_BASIC_LAND), true)",
])];

/// each opponent in turn order is asked, then searches (and shuffles) as the controller of the search
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let message = "Search your library for a basic land card and put it onto the battlefield tapped?";
        if choose_use(game, d, p, message)? {
            cx.run_as(game, d, &cx.effects[..1], p);
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}
