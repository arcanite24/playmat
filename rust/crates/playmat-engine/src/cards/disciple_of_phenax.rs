//! Disciple of Phenax: "When this creature enters, target player reveals a number of cards from their hand equal
//! to your devotion to black. You choose one of them. That player discards that card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DiscipleOfPhenax.DiscardCardYouChooseTargetEffect", apply)
    .args(&["one_of:DevotionCount.B"])];

/// `DiscardCardYouChooseTargetEffect(DevotionCount.B)`: the player picks that many cards to reveal (all of them when
/// the hand has no more), then you choose one of those, which they discard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let you = cx.controller();
    let n = devotion(game, you, Color::B) as usize;
    if n == 0 {
        return Some(());
    }
    let hand = game.player(p).hand.clone();
    let revealed = if hand.len() > n {
        let rule = item_rule(game, cx.item);
        choose_cards(game, d, p, &hand, &hand, n, n, &rule)?
    } else {
        hand
    };
    if revealed.is_empty() {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    let cards = revealed.iter().map(|c| (*c, Zone::Hand)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: true, cards });
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &revealed, &revealed, 1, 1, &rule)?;
    for c in &chosen {
        discard_card(game, p, *c);
    }
    if !chosen.is_empty() {
        game.discarded_cards();
    }
    Some(())
}
