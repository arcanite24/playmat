//! Valakut Awakening // Valakut Stoneforge: "Put any number of cards from your hand on the bottom of your library,
//! then draw that many cards plus one."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ValakutAwakening.ValakutAwakeningEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    // `player.choose(outcome, hand, TargetCardInHand(0, any))`: an empty hand isn't asked about
    let chosen = if hand.is_empty() {
        Vec::new()
    } else {
        let rule = item_rule(game, cx.item);
        choose_cards(game, d, you, &hand, &hand, 0, hand.len(), &rule)?
    };
    // `putCardsOnBottomOfLibrary(cards, …, true)`: in an order the player chooses
    if !put_on_library(game, d, you, &chosen, false, true) {
        return None;
    }
    draw(game, you, chosen.len() + 1);
    Some(())
}
