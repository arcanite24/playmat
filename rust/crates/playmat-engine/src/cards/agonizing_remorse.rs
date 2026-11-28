//! Agonizing Remorse: "Target opponent reveals their hand. You choose a nonland card from it or a card from their
//! graveyard. Exile that card. You lose 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AgonizingRemorse.AgonizingRemorseEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_NON_LAND"])
    .effects(&["new LoseLifeSourceControllerEffect(1)"])];

/// The reveal; "Exile a card from hand or graveyard?" (yes: the hand); a card of that zone (a nonland one from the
/// hand) exiled, nothing asked of an empty zone; then you lose 1
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
    let (shown, possible): (Vec<ObjId>, Vec<ObjId>) = if from_hand {
        let possible =
            hand.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c)).collect();
        (hand, possible)
    } else {
        let graveyard = game.player(p).graveyard.clone();
        (graveyard.clone(), graveyard)
    };
    if !shown.is_empty() {
        let rule = item_rule(game, cx.item);
        if let Some(&c) = choose_cards(game, d, you, &shown, &possible, 1, 1, &rule)?.first() {
            game.move_to(c, Zone::Exile);
        }
    }
    cx.run(game, d, &cx.effects[..1]);
    Some(())
}
