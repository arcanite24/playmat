//! Maddening Cacophony: "Each opponent mills eight cards. If this spell was kicked, instead each opponent mills half
//! their library, rounded up."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MaddeningCacophony.MaddeningCacophonyEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        let size = game.player(p).library.len();
        let n = if cx.item.kicked { size.div_ceil(2) } else { 8 };
        let top: Vec<ObjId> = game.player(p).library.iter().take(n).copied().collect();
        note(game, EventKind::Milled, Some(p), None, None, top.len() as i32);
        for c in top {
            game.move_to(c, Zone::Graveyard);
        }
    }
    Some(())
}
