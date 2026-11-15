//! Faller's Faithful: "When this creature enters, destroy up to one other target creature. If that creature wasn't
//! dealt damage this turn, its controller draws two cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FallersFaithful.FallersFaithfulEffect", apply)];

/// `getDealtDamageByThisTurn().isEmpty()`, read before it's destroyed; its controller then draws
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let damaged = game.obj(o).damaged_by.0 == game.turn && game.obj(o).damaged_by.1 != 0;
    let player = game.obj(o).controller;
    destroy(game, o);
    if !damaged {
        draw(game, player, 2);
    }
    Some(())
}
