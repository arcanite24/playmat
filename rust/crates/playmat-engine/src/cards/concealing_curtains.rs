//! Concealing Curtains // Revealing Eye: "When this creature transforms into Revealing Eye, target opponent reveals
//! their hand. You may choose a nonland card from it. If you do, that player discards that card, then draws a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ConcealingCurtains.RevealingEyeEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_NON_LAND"])];

/// The reveal; with no nonland card in it nothing more; else up to one nonland card chosen (`TargetCard(0, 1)`),
/// discarded, and that player draws
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let p = cx.player(0)?;
    let hand = game.player(p).hand.clone();
    if !hand.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = hand.iter().map(|c| (*c, Zone::Hand)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
    }
    let possible: Vec<ObjId> =
        hand.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c)).collect();
    if possible.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&c) = choose_cards(game, d, you, &hand, &possible, 0, 1, &rule)?.first() else { return Some(()) };
    discard_card(game, p, c);
    draw(game, p, 1);
    Some(())
}
