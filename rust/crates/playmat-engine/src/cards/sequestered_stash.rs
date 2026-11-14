//! Sequestered Stash: "{4}, {T}, Sacrifice Sequestered Stash: Mill five cards. Then you may put an artifact card from
//! your graveyard on top of your library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SequesteredStash.SequesteredStashEffect", apply)];

/// With an artifact card in your graveyard you may (`chooseUse`) choose one (`choose`, not targeted) to put on top
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let yard = game.player(you).graveyard.clone();
    let artifacts: Vec<ObjId> = yard.iter().copied().filter(|c| game.obj(*c).is(CardType::Artifact)).collect();
    if artifacts.is_empty()
        || !choose_use(game, d, you, "Put an artifact card from your graveyard to library?")?
    {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    if let Some(&c) = choose_cards(game, d, you, &yard, &artifacts, 1, 1, &rule)?.first() {
        to_library(game, c, true);
    }
    Some(())
}
