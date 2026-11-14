//! Gleeful Demolition: "Destroy target artifact. If you controlled that artifact, create three 1/1 red Phyrexian
//! Goblin creature tokens."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("GleefulDemolition.GleefulDemolitionEffect", apply).tokens(&["PhyrexianGoblinToken"])];

/// Whether you controlled it, the destroy, then the tokens (whether or not it was destroyed)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let mine = game.obj(o).controller == cx.controller();
    destroy(game, o);
    if mine {
        cx.create_tokens(game, d, &cx.tokens[0], 3);
    }
    Some(())
}
