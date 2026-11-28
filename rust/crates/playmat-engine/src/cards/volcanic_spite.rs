//! Volcanic Spite: "Volcanic Spite deals 3 damage to target creature, planeswalker, or battle. You may put a card
//! from your hand on the bottom of your library. If you do, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VolcanicSpite.VolcanicSpiteEffect", apply)];

/// asked only with a card in hand; then `TargetCardInHand` (required: a lone card is taken)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    if hand.is_empty() || !choose_use(game, d, you, "Put a card from your hand on the bottom of your library?")? {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&c) = choose_cards(game, d, you, &hand, &hand, 1, 1, &rule)?.first() else { return Some(()) };
    to_library(game, c, false);
    if game.obj(c).zone == Zone::Library {
        draw(game, you, 1);
    }
    Some(())
}
