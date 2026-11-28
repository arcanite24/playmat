//! Stillness in Motion: "At the beginning of your upkeep, mill three cards. Then if your library has no cards in it,
//! exile this enchantment and put five cards from your graveyard on top of your library in any order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("StillnessInMotion.StillnessInMotionEffect", apply)];

/// With an empty library: the enchantment (if it's still there) exiled, then five (or all) cards of your graveyard
/// chosen (`choose`, `TargetCardInYourGraveyard(n)`) and put on top in an order you choose
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !game.player(you).library.is_empty() {
        return Some(());
    }
    if cx.source_on_battlefield(game) {
        game.move_to(cx.source_id(), Zone::Exile);
    }
    let yard = game.player(you).graveyard.clone();
    let n = yard.len().min(5);
    if n < 1 {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &yard, &yard, n, n, &rule)?;
    if !put_on_library(game, d, you, &chosen, true, true) {
        return None;
    }
    Some(())
}
