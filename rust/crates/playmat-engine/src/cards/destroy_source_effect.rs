//! `DestroySourceEffect()` (framework): "destroy {this}": the source permanent, if it's still the one that put the
//! ability on the stack (`getSourcePermanentIfItStillExists`).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DestroySourceEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.source_on_battlefield(game) && !game.obj(cx.source_id()).phased_out {
        destroy(game, cx.source_id());
    }
    Some(())
}
