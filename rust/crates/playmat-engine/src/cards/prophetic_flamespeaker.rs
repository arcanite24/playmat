//! Prophetic Flamespeaker: "Double strike, trample. Whenever Prophetic Flamespeaker deals combat damage to a player,
//! exile the top card of your library. You may play it this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("PropheticFlamespeaker.PropheticFlamespeakerExileEffect", apply)];

/// The top card exiled; you may play it this turn (`PropheticFlamespeakerCastFromExileEffect`, `EndOfTurn`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile {
        let turn = game.turn;
        let o = game.obj_mut(card);
        o.may_play = Some((you, turn));
        o.may_play_end_step = false;
        o.may_play_while = None;
    }
    Some(())
}
