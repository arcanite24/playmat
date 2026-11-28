//! Yarok's Fenlurker: "When Yarok's Fenlurker enters the battlefield, each opponent exiles a card from their hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("YaroksFenlurker.YaroksFenlurkerEffect", apply)];

/// Each opponent with cards in hand chooses one (`TargetCardInHand`, `chooseTarget`); then each exiles theirs
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let mut chosen = Vec::new();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        let hand = game.player(p).hand.clone();
        if game.player(p).lost || hand.is_empty() {
            continue;
        }
        if let Some(&c) = choose_cards(game, d, p, &hand, &hand, 1, 1, &rule)?.first() {
            chosen.push(c);
        }
    }
    for c in chosen {
        game.move_to(c, Zone::Exile);
    }
    Some(())
}
