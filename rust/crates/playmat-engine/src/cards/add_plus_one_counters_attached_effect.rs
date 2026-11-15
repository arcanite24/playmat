//! `AddPlusOneCountersAttachedEffect(n)` (Forced Adaptation): "put n +1/+1 counters on enchanted creature".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AddPlusOneCountersAttachedEffect", apply).args(&["text"])];

/// The permanent the source is (or, last known, was) attached to, if it's still on the battlefield
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let amount = cx.arg_int(0)?;
    // `getPermanentOrLKIBattlefield`: an Aura returned to its owner's hand to pay for this (Krasis Incubation) is
    // read as it last was
    let src = game.obj(cx.source_id());
    let attached = if src.zone == Zone::Battlefield { src.attached_to } else { src.lki_attached_to };
    let Some((o, zcc)) = attached else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield || game.obj(o).zcc != zcc {
        return Some(());
    }
    add_counters_by(game, o, "+1/+1", amount, Some(cx.controller()));
    Some(())
}
