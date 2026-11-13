//! `RevealPutInHandLoseLifeEffect` (Dark Confidant): "reveal the top card of your library and put that card into
//! your hand. You lose life equal to its mana value."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RevealPutInHandLoseLifeEffect", apply)];

/// `ifYouDo` (the `true` argument) only changes the text: the card always goes to the hand
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = game.player(you).library.first().copied() else { return Some(()) };
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: vec![(card, Zone::Library)] });
    game.move_to(card, Zone::Hand);
    let mv = game.obj(card).def.mana_value() as i32;
    lose_life(game, you, mv);
    Some(())
}
