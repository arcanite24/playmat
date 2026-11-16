//! Pick the Brain: "Target opponent reveals their hand. You choose a nonland card from it and exile that card.
//! Delirium — If there are four or more card types among cards in your graveyard, search that player's graveyard,
//! hand, and library for any number of cards with the same name as the exiled card, exile those cards, then that
//! player shuffles."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PickTheBrain.PickTheBrainEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_A_NON_LAND"])];

/// With a hand: the reveal, a nonland card of it chosen (`TargetCard(HAND)`) and exiled; with delirium then,
/// `applySearchAndExile` for its name (`SearchTargetGraveyardHandLibraryForCardNameAndExileEffect`, any number)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let p = cx.player(0)?;
    let hand = game.player(p).hand.clone();
    if hand.is_empty() {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    let cards = hand.iter().map(|c| (*c, Zone::Hand)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
    let possible: Vec<ObjId> =
        hand.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c)).collect();
    if possible.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&card) = choose_cards(game, d, you, &hand, &possible, 1, 1, &rule)?.first() else { return Some(()) };
    let name = game.obj(card).def.name.clone();
    game.move_to(card, Zone::Exile);
    if !game.condition_holds(Some(&Condition::Delirium), you, cx.source_id()) {
        return Some(());
    }
    super::search_for_card_name_and_exile::search_and_exile_named(game, d, you, p, &name, usize::MAX, false)
}
