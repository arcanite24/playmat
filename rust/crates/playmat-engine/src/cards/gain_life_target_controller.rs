//! `GainLifeTargetControllerEffect` (Last Breath): "Its controller gains 4 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GainLifeTargetControllerEffect", apply)];

/// The first target's controller (`getControllerOfFirstTargetOrLKI`: a target an earlier effect of the ability
/// moved was its owner's) gains the amount
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.arg_int(0)?;
    let Some(Target::Obj(o)) = cx.legal.first().and_then(|g| g.first()).copied() else { return Some(()) };
    let p = game.obj(o).controller;
    gain_life(game, p, n);
    Some(())
}
