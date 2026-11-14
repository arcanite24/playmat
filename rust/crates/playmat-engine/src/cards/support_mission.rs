//! Support Mission: "Whenever a creature you control enters, put a quest counter on this enchantment. Put a +1/+1
//! counter on that creature. If this enchantment has five or more quest counters on it, instead put two +1/+1
//! counters on that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SupportMission.SupportMissionEffect", apply)];

/// the quest counters on the source as it is now (none when it's gone: nothing happens)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield || !cx.source_on_battlefield(game) {
        return Some(());
    }
    let n = if game.obj(cx.source_id()).counter("quest") >= 5 { 2 } else { 1 };
    put_counters(game, o, "+1/+1", n, cx.controller());
    Some(())
}
