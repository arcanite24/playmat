//! Infinite Obliteration: "Choose a creature card name. Search target opponent's graveyard, hand, and library for
//! any number of cards with that name and exile them. Then that player shuffles."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("InfiniteObliteration.InfiniteObliterationEffect", apply)
    .effects(&["new ChooseACardNameEffect(ChooseACardNameEffect.TypeOfName.CREATURE_NAME)"])];

/// `TypeOfName.CREATURE_NAME.getChoice`, then `applySearchAndExile` as Slaughter Games does (the same
/// `SearchTargetGraveyardHandLibraryForCardNameAndExileEffect(true, …)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.player(0).is_none() {
        return Some(());
    }
    cx.run(game, d, cx.effects);
    if d.stopped() {
        return None;
    }
    let Some(Handler { hook: Hook::Effect(search), .. }) = super::find("SlaughterGames.SlaughterGamesEffect") else {
        return Some(());
    };
    search(game, d, cx)
}
