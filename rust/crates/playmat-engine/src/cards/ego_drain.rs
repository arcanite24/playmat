//! Ego Drain: "If you don't control a Faerie, exile a card from your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EgoDrain.EgoDrainEffect", apply)];

/// Nothing with an empty hand or a Faerie; else a card of your hand (`TargetCardInHand`, `Player.choose`) exiled
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    let faerie = game.battlefield.iter().any(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Faerie"));
    if hand.is_empty() || faerie {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    if let Some(&c) = choose_cards(game, d, you, &hand, &hand, 1, 1, &rule)?.first() {
        game.move_to(c, Zone::Exile);
    }
    Some(())
}
