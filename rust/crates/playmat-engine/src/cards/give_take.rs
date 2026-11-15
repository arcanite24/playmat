//! Give // Take: "Take — Remove all +1/+1 counters from target creature you control. Draw that many cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GiveTake.TakeEffect", apply)];

/// Its +1/+1 counters off, then that many draws
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let n = game.obj(o).counter("+1/+1");
    if n <= 0 {
        return Some(());
    }
    let ob = game.obj_mut(o);
    ob.add_counters("+1/+1", -n);
    ob.counters.retain(|(_, k)| *k > 0);
    draw(game, cx.controller(), n as usize);
    Some(())
}
