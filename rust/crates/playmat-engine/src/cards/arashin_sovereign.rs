//! Arashin Sovereign: "When Arashin Sovereign dies, you may put it on the top or bottom of its owner's library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ArashinSovereign.ArashinSovereignEffect", apply)];

/// still in the graveyard: "Put … on top of it's owners library (otherwise on bottom)?", then there
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.source_id();
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let q = format!("Put {} on top of it's owners library (otherwise on bottom)?", game.obj(card).name());
    let top = choose_use(game, d, cx.controller(), &q)?;
    to_library(game, card, top);
    Some(())
}
