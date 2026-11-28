//! Oildeep Gearhulk: "When this creature enters, look at target player's hand. You may choose a card from it. If you
//! do, that player discards that card, then draws a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OildeepGearhulk.OildeepGearhulkEffect", apply)];

/// `lookAtCards`, then `choose(hand, TargetCard(0, 1))`; the discard and the draw only with a card chosen
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let you = cx.controller();
    let hand = game.player(p).hand.clone();
    // (an empty hand: nothing to choose from, nothing asked)
    if hand.is_empty() {
        return Some(());
    }
    let (source, zcc) = cx.source;
    game.looked_at.push(crate::state::LookedAt { viewer: you, source, zcc, bare: false, cards: hand.clone() });
    let rule = item_rule(game, cx.item);
    let Some(&card) = choose_cards(game, d, you, &hand, &hand, 0, 1, &rule)?.first() else { return Some(()) };
    if game.obj(card).zone != Zone::Hand {
        return Some(());
    }
    discard_card(game, p, card);
    game.discarded_cards();
    draw(game, p, 1);
    Some(())
}
