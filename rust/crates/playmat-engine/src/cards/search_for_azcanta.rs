//! Search for Azcanta // Azcanta, the Sunken Ruin: "At the beginning of your upkeep, look at the top card of your
//! library. You may put it into your graveyard. Then if you have seven or more cards in your graveyard, you may
//! transform Search for Azcanta."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SearchForAzcanta.SearchForAzcantaEffect", apply)];

/// With the enchantment still there and seven or more cards in your graveyard, you may (`chooseUse`) transform it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let you = cx.controller();
    if game.player(you).graveyard.len() < 7 {
        return Some(());
    }
    let src = cx.source_id();
    let message = format!("Transform {}?", game.obj(src).name());
    if choose_use(game, d, you, &message)? {
        transform(game, src);
    }
    Some(())
}
