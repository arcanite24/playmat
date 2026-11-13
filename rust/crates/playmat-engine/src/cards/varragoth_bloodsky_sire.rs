//! Varragoth, Bloodsky Sire: "Boast — {1}{B}: Target player searches their library for a card, then shuffles and
//! puts that card on top."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VarragothBloodskySire.VarragothBloodskySireEffect", apply)];

/// The target player searches (`TargetCardInLibrary()`: one card), shuffles, then puts the card found on top
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let found = search_library(game, d, p, &crate::card::Filter::default(), 1, 1, cx.source_id())?;
    let mut library = game.player(p).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(p).library = library;
    if !found.is_empty() && !put_cards(game, d, p, &found, crate::card::PutCards::TopAny) {
        return None;
    }
    Some(())
}
