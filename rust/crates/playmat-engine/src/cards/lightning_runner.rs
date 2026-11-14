//! Lightning Runner: "Whenever Lightning Runner attacks, you get {E}{E}, then you may pay eight {E}. If you pay,
//! untap all creatures you control, and after this phase, there is an additional combat phase."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LightningRunner.LightningRunnerEffect", apply).effects(&[
    "new UntapAllControllerEffect(StaticFilters.FILTER_CONTROLLED_CREATURES)",
    "new AdditionalCombatPhaseEffect()",
])];

/// Two energy; then, with more than seven, you're asked ("Pay eight {E} to use this? "), and paying untaps your
/// creatures and adds a combat phase after this one
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    add_energy(game, you, 2);
    if energy(game, you) <= 7 {
        return Some(());
    }
    if !choose_use(game, d, you, "Pay eight {E} to use this? ")? {
        return Some(());
    }
    add_energy(game, you, -8);
    cx.run_on(game, d, cx.effects, Vec::new());
    Some(())
}
