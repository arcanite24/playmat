//! Mindleech Ghoul: "Exploit. When Mindleech Ghoul exploits a creature, each opponent exiles a card from their hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MindleechGhoul.MindleechGhoulEffect", apply)];

/// each opponent with cards in hand chooses one (`TargetCardInHand`, `Player.choose`); then they're exiled together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let mut cards = Vec::new();
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost || game.player(p).hand.is_empty() {
            continue;
        }
        let hand = game.player(p).hand.clone();
        if let Some(&c) = choose_cards(game, d, p, &hand, &hand, 1, 1, &rule)?.first() {
            cards.push(c);
        }
    }
    for c in cards {
        game.move_to(c, Zone::Exile);
    }
    Some(())
}
