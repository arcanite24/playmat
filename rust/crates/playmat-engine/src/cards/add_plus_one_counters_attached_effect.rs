//! `AddPlusOneCountersAttachedEffect(n)` (Forced Adaptation): "put n +1/+1 counters on enchanted creature".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AddPlusOneCountersAttachedEffect", apply).args(&["text"])];

/// The permanent the source is (or, last known, was) attached to, if it's still on the battlefield
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let amount = cx.arg_int(0)?;
    let Some((o, zcc)) = game.obj(cx.source_id()).attached_to else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield || game.obj(o).zcc != zcc {
        return Some(());
    }
    add_counters_by(game, o, "+1/+1", amount, Some(cx.controller()));
    Some(())
}
