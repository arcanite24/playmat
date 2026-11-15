//! Progenitor Exarch: "When Progenitor Exarch enters, incubate 3 X times. {T}: Transform target Incubator token you
//! control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ProgenitorExarch.ProgenitorExarchEffect", apply).effects(&["new IncubateEffect(3)"])];

/// `IncubateEffect.doIncubate(3)` X times (X paid for it, the enters trigger's)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for _ in 0..cx.item.x.max(0) {
        cx.run(game, d, &cx.effects[..1]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
