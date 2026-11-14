//! Celestial Regulator: "When Celestial Regulator enters, choose target creature you don't control and tap it. If you
//! control a creature with a counter on it, the chosen creature doesn't untap during its controller's next untap
//! step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CelestialRegulator.CelestialRegulatorEffect", apply)
    .effects(&["new TapTargetEffect()", "new DontUntapInControllersNextUntapStepTargetEffect()"])];

/// the creature with a counter is looked for after the tap
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.obj(0).is_none() {
        return Some(());
    }
    cx.run(game, d, &cx.effects[..1]);
    let you = cx.controller();
    let countered = game.battlefield.iter().any(|p| {
        game.obj(*p).controller == you
            && game.has_type(*p, CardType::Creature)
            && game.obj(*p).counters.iter().any(|(_, n)| *n > 0)
    });
    if countered {
        cx.run(game, d, &cx.effects[1..2]);
    }
    Some(())
}
