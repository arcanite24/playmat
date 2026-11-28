//! Ancestor Dragon: "Whenever one or more creatures you control attack, you gain 1 life for each attacking
//! creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AncestorDragon.AncestorDragonEffect", apply)];

/// `game.getCombat().getAttackers().size()`
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = attackers(game).len() as i32;
    gain_life(game, cx.controller(), n);
    Some(())
}
