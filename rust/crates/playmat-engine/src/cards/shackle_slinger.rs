//! Shackle Slinger: "Whenever you cast your second spell each turn, choose target creature an opponent controls.
//! If it's tapped, put a stun counter on it. Otherwise, tap it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShackleSlinger.ShackleSlingerEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    if game.obj(o).tapped {
        add_counters_by(game, o, "stun", 1, Some(cx.controller()));
    } else {
        tap(game, o);
    }
    Some(())
}
