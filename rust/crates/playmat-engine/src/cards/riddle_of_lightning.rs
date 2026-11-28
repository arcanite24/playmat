//! Riddle of Lightning: "Choose any target. Scry 3, then reveal the top card of your library. Riddle of Lightning
//! deals damage equal to that card's mana value to that permanent or player."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RiddleOfLightning.RiddleOfLightningEffect", apply)];

/// the top card's mana value, to the target
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(top) = game.player(cx.controller()).library.first().copied() else { return Some(()) };
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: vec![(top, Zone::Library)] });
    let n = game.obj(top).def.mana_value() as i32;
    if let Some(o) = cx.obj(0) {
        deal_damage(game, cx.source_id(), Target::Obj(o), n, false);
    } else if let Some(p) = cx.player(0) {
        deal_damage(game, cx.source_id(), Target::Player(p), n, false);
    }
    Some(())
}
