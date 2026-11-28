//! Thassa's Oracle: "When this creature enters, look at the top X cards of your library, where X is your devotion
//! to blue. Put up to one of them on top of your library and the rest on the bottom of your library in a random
//! order. If X is greater than or equal to the number of cards in your library, you win the game."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ThassasOracle.ThassasOracleEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    // an empty library: you win before anything is looked at
    if game.player(you).library.is_empty() {
        cx.run(game, d, &[EffectDef::WinGameSourceControllerEffect]);
        return Some(());
    }
    let x = devotion(game, you, Color::U) as usize;
    let cards: Vec<ObjId> = game.player(you).library.iter().take(x).copied().collect();
    // `player.choose(outcome, cards, TargetCardInLibrary(0, 1))`: nothing to choose from isn't asked
    let chosen = if cards.is_empty() {
        Vec::new()
    } else {
        let rule = item_rule(game, cx.item);
        choose_cards(game, d, you, &cards, &cards, 0, 1, &rule)?
    };
    let rest: Vec<ObjId> = cards.iter().copied().filter(|c| !chosen.contains(c)).collect();
    // `putCardsOnTopOfLibrary(…, false)`, then `putCardsOnBottomOfLibrary(…, false)`: each in a random order
    if !put_on_library(game, d, you, &chosen, true, false) || !put_on_library(game, d, you, &rest, false, false) {
        return None;
    }
    if x >= game.player(you).library.len() {
        cx.run(game, d, &[EffectDef::WinGameSourceControllerEffect]);
    }
    Some(())
}
