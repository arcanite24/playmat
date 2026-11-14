//! Sweep Away: "Return target creature to its owner's hand. If that creature is attacking, you may put it on top of
//! its owner's library instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SweepAway.SweepAwayEffect", apply)
    .effects(&["new PutOnLibraryTargetEffect(true)", "new ReturnToHandTargetEffect()"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let top = game.obj(o).attacking.is_some()
        && choose_use(
            game,
            d,
            cx.controller(),
            &format!("Put {} on top of its owner's library (otherwise return to hand)?", game.obj(o).name()),
        )?;
    cx.run(game, d, if top { &cx.effects[..1] } else { &cx.effects[1..2] });
    Some(())
}
