//! Mirkwood Nurturer: "When this creature enters, return up to one other target permanent you control to its owner's
//! hand. If you do, put a +1/+1 counter on this creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MirkwoodNurturer.MirkwoodNurturerEffect", apply)];

/// the target returned, then the counter on the source while it's still on the battlefield
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    game.move_to(o, Zone::Hand);
    if game.obj(o).zone != Zone::Hand {
        return Some(());
    }
    if cx.source_on_battlefield(game) {
        put_counters(game, cx.source_id(), "+1/+1", 1, cx.controller());
    }
    Some(())
}
