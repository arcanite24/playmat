//! Never Happened: "Target opponent reveals their hand. You choose a nonland card from that player's graveyard or
//! hand and exile it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NeverHappened.NeverHappenedEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_NON_LAND"])];

/// The reveal; "Exile a card from hand or graveyard?" (yes: the hand); a nonland card of that zone exiled, nothing
/// asked of an empty zone
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let p = cx.player(0)?;
    let hand = game.player(p).hand.clone();
    if !hand.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = hand.iter().map(|c| (*c, Zone::Hand)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
    }
    let q = Query::YesNo { message: "Exile a card from hand or graveyard?".into() };
    let from_hand = loop {
        match ask(game, d, you, &q) {
            Answer::Bool(b) => break b,
            Answer::Stop => return None,
            _ => {}
        }
    };
    let shown = if from_hand { hand } else { game.player(p).graveyard.clone() };
    if shown.is_empty() {
        return Some(());
    }
    let possible: Vec<ObjId> =
        shown.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c)).collect();
    let rule = item_rule(game, cx.item);
    if let Some(&c) = choose_cards(game, d, you, &shown, &possible, 1, 1, &rule)?.first() {
        game.move_to(c, Zone::Exile);
    }
    Some(())
}
