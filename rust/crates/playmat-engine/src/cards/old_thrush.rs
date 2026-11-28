//! Old Thrush: "When this creature enters, you gain 2 life. You may search your library for a basic land card, reveal
//! it, then shuffle and put that card on top."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OldThrush.OptionalOneShotEffect", apply).args(&["effect"])];

/// `OptionalOneShotEffect`: asked with its own text ("you may " and the effect's, as a sentence), then the effect
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let message =
        "You may search your library for a basic land card, reveal it, then shuffle and put that card on top.";
    if choose_use(game, d, cx.controller(), message)? {
        cx.run(game, d, cx.effects);
    }
    Some(())
}
