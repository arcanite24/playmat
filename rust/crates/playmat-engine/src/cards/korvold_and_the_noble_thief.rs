//! Korvold and the Noble Thief: "(As this Saga enters and after your draw step, add a lore counter.) I, II — Create a
//! Treasure token. III — Exile the top three cards of target opponent's library. You may play those cards this
//! turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KorvoldAndTheNobleThief.KorvoldAndTheNobleThiefEffect", apply)];

/// the opponent's top three cards into exile, each one there playable by you this turn (`makeCardPlayable`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let cards: Vec<ObjId> = game.player(opponent).library.iter().take(3).copied().collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let turn = game.turn;
    let exiled: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    for c in exiled {
        game.obj_mut(c).may_play = Some((you, turn));
    }
    Some(())
}
