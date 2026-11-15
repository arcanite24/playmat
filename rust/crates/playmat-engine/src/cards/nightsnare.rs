//! Nightsnare: "Target opponent reveals their hand. You may choose a nonland card from it. If you do, that player
//! discards that card. If you don't, that player discards two cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Nightsnare.NightsnareDiscardEffect", apply)];

/// With a hand: it's revealed, then "choose a card to discard?"; no: they discard two (their choice); yes: you
/// choose a nonland card from it (`TargetCard(1, HAND, nonland)`), which they discard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let hand = game.player(p).hand.clone();
    if hand.is_empty() {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    let cards = hand.iter().map(|c| (*c, Zone::Hand)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
    let message = format!("Choose a card to discard? (Otherwise {} has to discard 2 cards).", log::player(p));
    if !choose_use(game, d, you, &message)? {
        discard(game, d, p, 2);
        return (!d.stopped()).then_some(());
    }
    let nonland: Vec<ObjId> = hand.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect();
    if nonland.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, you, &hand, &nonland, 1, 1, &item_rule(game, cx.item))?;
    if let Some(&c) = chosen.first() {
        discard_card(game, p, c);
        game.discarded_cards();
    }
    Some(())
}
