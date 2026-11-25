//! Through the Forest Gate: "Look at the top twenty cards of your library, put any number of land cards from among
//! them onto the battlefield tapped, then shuffle. You gain 8 life." (the shuffle and the life are its own effects)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ThroughTheForestGate.ThroughTheForestGateEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_LANDS"])];

/// the top twenty, any number of the lands among them chosen (`TargetCardInLibrary(0, MAX, lands)`), put onto the
/// battlefield tapped together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.first()?;
    let top: Vec<ObjId> = game.player(you).library.iter().take(20).copied().collect();
    if top.is_empty() {
        return Some(());
    }
    let possible: Vec<ObjId> =
        top.iter().copied().filter(|c| game.filter_matches(filter, you, Some(cx.source_id()), *c)).collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &top, &possible, 0, possible.len(), &rule)?;
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    Some(())
}
