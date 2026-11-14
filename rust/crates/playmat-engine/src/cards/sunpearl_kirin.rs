//! Sunpearl Kirin: "When this creature enters, return up to one other target nonland permanent you control to its
//! owner's hand. If it was a token, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SunpearlKirin.SunpearlKirinEffect", apply)];

/// To the hand; a token then ceases to exist (`processAction`) and you draw
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let token = game.obj(o).token;
    game.move_to(o, Zone::Hand);
    if token {
        draw(game, cx.controller(), 1);
    }
    Some(())
}
