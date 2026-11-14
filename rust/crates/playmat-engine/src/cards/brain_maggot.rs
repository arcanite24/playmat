//! Brain Maggot: "When Brain Maggot enters the battlefield, target opponent reveals their hand and you choose a
//! nonland card from it. Exile that card until Brain Maggot leaves the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BrainMaggot.BrainMaggotExileEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_NON_LAND"])];

/// The opponent's hand revealed (`getPermanentOrLKIBattlefield`: even after this left); with a nonland card in it,
/// you choose one (`TargetCard(Zone.HAND, nonland)`), exiled until this leaves (`ExileUntilSourceLeavesEffect`: only
/// while this is still on the battlefield)
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
    let Some(&card) = choose_cards(game, d, you, &hand, &possible, 1, 1, "nonland card to exile")?.first() else {
        return Some(());
    };
    if cx.source_on_battlefield(game) {
        let (src, zcc) = cx.source;
        game.move_to(card, Zone::Exile);
        if game.obj(card).zone == Zone::Exile {
            game.exiled_until_leaves.push((src, zcc, card));
        }
    }
    Some(())
}
