//! Nashi, Searcher in the Dark: "Whenever Nashi deals combat damage to a player, you mill that many cards. You may
//! put any number of legendary and/or enchantment cards from among them into your hand. If you put no cards into
//! your hand this way, put a +1/+1 counter on Nashi."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NashiSearcherInTheDark.NashiSearcherInTheDarkEffect", apply)];

/// You mill the damage dealt (the trigger's X); you choose any number of the legendary or enchantment cards milled
/// (`TargetCard(0, MAX)`, not targeted) for your hand; with none put there, a +1/+1 counter on Nashi if it's still
/// the permanent
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let amount = cx.item.x;
    if amount < 1 {
        return Some(());
    }
    let milled: Vec<ObjId> = game.player(you).library.iter().take(amount as usize).copied().collect();
    note(game, EventKind::Milled, Some(you), None, None, milled.len() as i32);
    for c in &milled {
        game.move_to(*c, Zone::Graveyard);
    }
    let milled: Vec<ObjId> = milled.into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    let possible: Vec<ObjId> = milled
        .iter()
        .copied()
        .filter(|c| {
            game.obj(*c).def.supertypes.iter().any(|s| s == "LEGENDARY") || game.has_type(*c, CardType::Enchantment)
        })
        .collect();
    let mut to_hand = Vec::new();
    // asked whenever cards were milled, even with none of them choosable (`choose(outcome, cards, target)`)
    if !milled.is_empty() {
        let rule = item_rule(game, cx.item);
        to_hand = choose_cards(game, d, you, &milled, &possible, 0, 99, &rule)?;
        if !to_hand.is_empty() {
            put_cards(game, d, you, &to_hand, crate::card::PutCards::Hand);
            if d.stopped() {
                return None;
            }
        }
    }
    if !to_hand.iter().any(|c| game.obj(*c).zone == Zone::Hand) && cx.source_on_battlefield(game) {
        put_counters(game, cx.source_id(), "+1/+1", 1, you);
    }
    Some(())
}
