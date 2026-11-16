//! Captain Jean-Luc Picard: "Other creatures you control get +1/+1. Whenever you cast a creature or Spacecraft
//! spell, create a 1/1 red Officer creature token. Tap seven untapped Officers you control: Search your library for a
//! Spacecraft card, put it onto the battlefield with seven charge counters on it, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CaptainJeanLucPicard.CaptainJeanLucPicardEffect", apply)];

/// `searchLibrary(TargetCardInLibrary(a Spacecraft card))`; the card found goes onto the battlefield, its seven
/// charge counters put as it arrives (`setEnterWithCounters`); then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = Filter(vec![Pred::Subtype { s: "Spacecraft".into() }]);
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(&c) = found.first() {
        if game.obj(c).zone == Zone::Library {
            enter_battlefield(game, d, c, you, false);
            if game.obj(c).zone == Zone::Battlefield {
                add_counters_by(game, c, "charge", 7, Some(you));
            }
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
