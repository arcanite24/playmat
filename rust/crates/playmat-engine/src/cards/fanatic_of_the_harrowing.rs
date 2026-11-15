//! Fanatic of the Harrowing: "When Fanatic of the Harrowing enters, each player discards a card. If you discarded a
//! card this way, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FanaticOfTheHarrowing.FanaticOfTheHarrowingEffect", apply)];

/// each player in range from you discards one, then you draw if you discarded
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut flag = false;
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let discarded = discard(game, d, p, 1);
        if d.stopped() {
            return None;
        }
        if !discarded.is_empty() && p == you {
            flag = true;
        }
    }
    if flag {
        draw(game, you, 1);
    }
    Some(())
}
