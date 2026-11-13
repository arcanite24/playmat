//! Doppelgang: "For each of X target permanents, create X tokens that are copies of that permanent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Doppelgang.DoppelgangEffect", apply)];

/// X token copies of each target still on the battlefield, target by target
/// (`CreateTokenCopyTargetEffect(null, null, false, X)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x;
    if x < 1 {
        return Some(());
    }
    let copy: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "CreateTokenCopyTargetEffect", "number": x})).ok()?;
    for o in cx.objs(0) {
        if game.obj(o).zone == Zone::Battlefield {
            cx.run_on(game, d, std::slice::from_ref(&copy), vec![Target::Obj(o)]);
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}
