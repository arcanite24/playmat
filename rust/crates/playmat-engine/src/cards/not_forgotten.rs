//! Not Forgotten: "Put target card from a graveyard on the top or bottom of its owner's library. Create a 1/1 white
//! Spirit creature token with flying."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("NotForgotten.NotForgottenEffect", apply).tokens(&["SpiritWhiteToken"])];

/// `chooseUse` top (yes) or bottom, then the token only when the card moved
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let c = cx.obj(0)?;
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    let message = format!("Put {} on top of it's owners library (otherwise on bottom)?", game.obj(c).name());
    let top = choose_use(game, d, cx.controller(), &message)?;
    to_library(game, c, top);
    if game.obj(c).zone == Zone::Library {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
    }
    (!d.stopped()).then_some(())
}
