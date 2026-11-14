//! Lilysplash Mentor: "{1}{G}{U}: Exile another target creature you control, then return it to the battlefield
//! under its owner's control with a +1/+1 counter on it. Activate only as a sorcery."
//! (`ExileThenReturnTargetEffect(false, false).withEnterWithCounters(P1P1)`)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LilysplashMentor.ExileThenReturnTargetEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    super::essence_flux::flicker(game, d, cx, Some(("+1/+1", 1)));
    Some(())
}
