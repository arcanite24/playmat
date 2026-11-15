//! Case the Joint: "Draw two cards, then look at the top card of each player's library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CaseTheJoint.CaseTheJointEffect", apply)];

/// each player in range, in seat order: the controller looks at the top card of their library (views only)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for p in 0..game.seats() {
        let Some(top) = game.player(p).library.first().copied() else { continue };
        let zcc = game.obj(cx.source_id()).zcc;
        game.looked_at.push(crate::state::LookedAt { viewer: you, source: cx.source_id(), zcc, bare: true, cards: vec![top] });
    }
    Some(())
}
