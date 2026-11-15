//! Warlock Class: "(Gain the next level as a sorcery to add its ability.) At the beginning of your end step, if a
//! creature died this turn, each opponent loses 1 life. {1}{B}: Level 2. When this Class becomes level 2, look at the
//! top three cards of your library. Put one of them into your hand and the rest into your graveyard. {6}{B}: Level 3.
//! At the beginning of your end step, each opponent loses life equal to the life they lost this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WarlockClass.WarlockClassEffect", apply)];

/// `PlayerLostLifeWatcher`: each opponent, in seat order, loses as much life as they've lost this turn (when that's
/// more than none)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for s in 0..game.seats() {
        if s == you {
            continue;
        }
        let lost = game.player(s).life_lost;
        if lost > 0 {
            lose_life(game, s, lost);
        }
    }
    Some(())
}
