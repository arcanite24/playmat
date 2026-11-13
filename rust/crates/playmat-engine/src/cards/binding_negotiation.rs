//! Binding Negotiation: "Target opponent reveals their hand. You may choose a nonland card from it. If you do, they
//! discard it. Otherwise, you may put a face-up exiled card they own into their graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BindingNegotiation.BindingNegotiationEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_NON_LAND"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let hand = game.player(p).hand.clone();
    if !hand.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = hand.iter().map(|c| (*c, Zone::Hand)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
    }
    let possible: Vec<ObjId> =
        hand.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c)).collect();
    let rule = item_rule(game, cx.item);
    // `choose(outcome, cards, TargetCard(0, 1))`: an empty hand isn't asked
    let chosen = if hand.is_empty() { Vec::new() } else { choose_cards(game, d, you, &hand, &possible, 0, 1, &rule)? };
    if let Some(&c) = chosen.first() {
        discard_card(game, p, c);
        return Some(());
    }
    // `TargetCardInExile(0, 1)` of their face-up cards: asked even with nothing to pick
    let exiled: Vec<ObjId> = game
        .exile
        .iter()
        .copied()
        .filter(|c| game.obj(*c).owner == p && !game.obj(*c).face_down)
        .collect();
    let chosen = choose_cards(game, d, you, &exiled, &exiled, 0, 1, &rule)?;
    for c in chosen {
        game.move_to(c, Zone::Graveyard);
    }
    Some(())
}
