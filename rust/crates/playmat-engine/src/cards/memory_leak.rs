//! Memory Leak: "Target opponent reveals their hand. You choose a nonland card from that player's graveyard or hand
//! and exile it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MemoryLeak.MemoryLeakEffect", apply)];

/// The hand revealed; "Hand" or "Graveyard" (`chooseUse`); then `choose(TargetCard(zone, nonland))` from that zone,
/// asked unless it's empty, and the card exiled
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let hand = game.player(p).hand.clone();
    if !hand.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = hand.iter().map(|c| (*c, Zone::Hand)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
    }
    let from_hand = choose_use(game, d, you, "Exile a card from hand or graveyard?")?;
    let cards = if from_hand { hand } else { game.player(p).graveyard.clone() };
    if cards.is_empty() {
        return Some(());
    }
    let possible: Vec<ObjId> = cards.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect();
    let rule = item_rule(game, cx.item);
    if let Some(&c) = choose_cards(game, d, you, &cards, &possible, 1, 1, &rule)?.first() {
        game.move_to(c, Zone::Exile);
    }
    Some(())
}
