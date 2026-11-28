//! Dreamdew Entrancer: "When this creature enters, tap up to one target creature and put three stun counters on it.
//! If you control that creature, draw two cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DreamdewEntrancer.DreamdewEntrancerEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone == Zone::Battlefield && game.obj(o).controller == you {
        draw(game, you, 2);
    }
    Some(())
}
