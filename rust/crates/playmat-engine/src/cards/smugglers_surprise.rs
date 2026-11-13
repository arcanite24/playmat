//! Smuggler's Surprise's second mode: "You may put up to two creature cards from your hand onto the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SmugglersSurprise.SmugglersSurpriseEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_CREATURES"])];

/// `TargetCardInHand(0, 2)`, one `choose` (asked even with nothing to pick), then one `moveCards`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    let possible: Vec<ObjId> =
        hand.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c)).collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &hand, &possible, 0, 2, &rule)?;
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
