//! Into the Fire: "Choose one — • Into the Fire deals 2 damage to each attacking creature. • Put any number of cards
//! from your hand on the bottom of your library, then draw that many cards plus one."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("IntoTheFire.IntoTheFireEffect", apply)];

/// `choose` any number of cards from your hand (not asked with none), `putCardsOnBottomOfLibrary` in any order,
/// then the draws
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    let chosen = if hand.is_empty() {
        Vec::new()
    } else {
        let rule = item_rule(game, cx.item);
        choose_cards(game, d, you, &hand, &hand, 0, usize::MAX, &rule)?
    };
    if !put_on_library(game, d, you, &chosen, false, true) {
        return None;
    }
    draw(game, you, chosen.len() + 1);
    Some(())
}
