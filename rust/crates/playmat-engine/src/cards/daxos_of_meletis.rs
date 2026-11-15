//! Daxos of Meletis: "Daxos of Meletis can't be blocked by creatures with power 3 or greater. Whenever Daxos of
//! Meletis deals combat damage to a player, exile the top card of that player's library. You gain life equal to that
//! card's mana value. Until end of turn, you may cast that card and you may spend mana as though it were mana of any
//! color to cast that spell."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DaxosOfMeletis.DaxosOfMeletisEffect", apply)];

/// The damaged player's top card exiled, life gained equal to its mana value, and the card castable by you this turn
/// with mana as though of any color (`CardUtil.makeCardPlayable(…, useCastSpellOnly, EndOfTurn, anyColor)`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let Some(card) = game.player(p).library.first().copied() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    let mv = game.obj(card).def.mana_value() as i32;
    gain_life(game, you, mv);
    if game.obj(card).zone == Zone::Exile && !game.obj(card).is(CardType::Land) {
        let turn = game.turn;
        let o = game.obj_mut(card);
        o.may_play = Some((you, turn));
        o.may_play_end_step = false;
        o.may_play_while = None;
        o.any_mana = true;
        o.cast_only = true;
    }
    Some(())
}
