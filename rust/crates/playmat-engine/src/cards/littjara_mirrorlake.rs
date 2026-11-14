//! Littjara Mirrorlake: "{2}{G}{G}{U}, {T}, Sacrifice Littjara Mirrorlake: Create a token that's a copy of target
//! creature you control, except it enters with an additional +1/+1 counter on it. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LittjaraMirrorlake.LittjaraMirrorlakeEffect", apply)];

/// `CreateTokenCopyTargetEffect` of the target (or as it last was), then a +1/+1 counter on each token made
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let of = match cx.item.targets.first().and_then(|t| t.first()) {
        Some(Target::Obj(o)) => *o,
        _ => return Some(()),
    };
    let made = token_copy_modified(game, d, of, you, 1, &|_: &mut CardDef| {});
    if d.stopped() {
        return None;
    }
    for t in made {
        if game.obj(t).zone == Zone::Battlefield {
            put_counters(game, t, "+1/+1", 1, you);
        }
    }
    Some(())
}
