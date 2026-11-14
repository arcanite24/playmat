//! Fire Prophecy: "Fire Prophecy deals 3 damage to target creature. You may put a card from your hand on the bottom
//! of your library. If you do, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FireProphecy.FireProphecyEffect", apply)];

/// With a card in hand you may (`chooseUse`) put one (`player.choose`, `TargetCardInHand`) on the bottom, then draw
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if game.player(you).hand.is_empty() {
        return Some(());
    }
    if !choose_use(game, d, you, "Put a card from your hand on the bottom of your library?")? {
        return Some(());
    }
    let hand = game.player(you).hand.clone();
    let rule = item_rule(game, cx.item);
    let Some(&c) = choose_cards(game, d, you, &hand, &hand, 1, 1, &rule)?.first() else { return Some(()) };
    to_library(game, c, false);
    draw(game, you, 1);
    Some(())
}
