//! Tinybones, Trinket Thief: "At the beginning of each end step, if an opponent discarded a card this turn, you draw a
//! card and you lose 1 life. {4}{B}{B}: Each opponent with no cards in hand loses 10 life."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("TinybonesTrinketThief.TinybonesTrinketThiefCondition", opponent_discarded),
    Handler::effect("TinybonesTrinketThief.TinybonesTrinketThiefEffect", empty_handed_lose),
];

/// `TinybonesTrinketThiefWatcher`: an opponent of yours discarded a card this turn (`DISCARDED_CARD`)
fn opponent_discarded(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.discarded.iter().any(|s| *s != you)
}

/// Each opponent, in turn order, with an empty hand loses 10 life
fn empty_handed_lose(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).hand.is_empty() {
            lose_life(game, p, 10);
        }
    }
    Some(())
}
