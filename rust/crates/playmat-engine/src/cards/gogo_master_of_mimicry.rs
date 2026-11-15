//! Gogo, Master of Mimicry: "{X}{X}, {T}: Copy target activated or triggered ability you control X times. You may
//! choose new targets for the copies. This ability can't be copied and X can't be 0."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GogoMasterOfMimicry.GogoMasterOfMimicryCopyEffect", apply)
    .effects(&["new CopyTargetStackObjectEffect()"])];

/// `stackObject.createCopyOnStack(…, true, X)`: X copies, new targets asked for each
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x;
    if x < 1 || cx.obj(0).is_none() {
        return Some(());
    }
    for _ in 0..x {
        cx.run(game, d, &cx.effects[..1]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
