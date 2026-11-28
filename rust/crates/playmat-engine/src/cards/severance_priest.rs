//! Severance Priest: "When this creature enters, target opponent reveals their hand. You may choose a nonland card
//! from it. If you do, exile that card." Its leaves trigger is `CreateXXTokenExiledEffect` (the cards exiled for it).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SeverancePriest.SeverancePriestEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_NON_LAND"])];

/// An empty hand: nothing. Else the hand revealed, then up to one nonland card of it chosen
/// (`TargetCard(0, 1, Zone.HAND, nonland)`, nothing asked with none) and exiled into the source's exile zone
/// (`CardUtil.getExileZoneId(game, source)`), which the leaves trigger reads
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
    let Some(&card) = choose_cards(game, d, you, &hand, &possible, 0, 1, "nonland card")?.first() else {
        return Some(());
    };
    let (src, zcc) = cx.source;
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile {
        game.exiled_for_source.push((src, zcc, card));
    }
    Some(())
}
