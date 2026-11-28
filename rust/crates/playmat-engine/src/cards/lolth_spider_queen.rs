//! Lolth, Spider Queen's emblem: "Whenever an opponent is dealt combat damage by one or more creatures you control, if
//! that player lost less than 8 life this turn, they lose life equal to the difference."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("LolthSpiderQueenEmblemCondition", lost_less_than_eight),
    Handler::effect("LolthSpiderQueenEmblemEffect", lose_difference),
];

/// `PlayerLostLifeWatcher.getLifeLost(that player) < 8`, that player the damaged one: in a duel, your opponent
fn lost_less_than_eight(game: &Game, you: Seat, _: ObjId) -> bool {
    game.player(game.opponent(you)).life_lost < 8
}

/// the target pointer's player (the damaged one) loses `8 - life lost this turn`, at least 0
fn lose_difference(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let n = (8 - game.player(p).life_lost).max(0);
    if n > 0 {
        lose_life(game, p, n);
    }
    Some(())
}
