//! Whisk Away: "Put target attacking or blocking creature on top of its owner's library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WhiskAway.WhiskAwayEffect", apply)];

/// `putCardsOnTopOfLibrary` of the creature (one card: no order asked)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    if !put_on_library(game, d, cx.controller(), &[t], true, true) {
        return None;
    }
    Some(())
}
