//! Feed the Infection: "You draw three cards and you lose 3 life. Corrupted — Each opponent who has three or more
//! poison counters loses 3 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FeedTheInfection.FeedTheInfectionEffect", apply)];

/// Each opponent, in turn order, with three or more poison counters
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if !game.player(p).lost && poison(game, p) >= 3 {
            lose_life(game, p, 3);
        }
    }
    Some(())
}
