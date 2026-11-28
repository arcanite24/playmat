//! Dark Deal: "Each player discards all the cards in their hand, then draws that many cards minus one."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DarkDeal.DarkDealEffect", apply)];

/// Each player in range from you discards their hand, then each of them draws one fewer, in the same order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut draws = Vec::new();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let n = game.player(p).hand.len();
        if n > 0 {
            discard(game, d, p, n as i32);
            if d.stopped() {
                return None;
            }
            draws.push((p, n - 1));
        }
    }
    for (p, n) in draws {
        if n > 0 {
            draw(game, p, n);
        }
    }
    Some(())
}
