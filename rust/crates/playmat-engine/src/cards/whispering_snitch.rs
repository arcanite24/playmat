//! Whispering Snitch: "Whenever you surveil for the first time each turn, Whispering Snitch deals 1 damage to each
//! opponent and you gain 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "WhisperingSnitch.WhisperingSnitchTriggeredAbility",
        "new SurveilTriggeredAbility(new WhisperingSnitchEffect())",
        first_surveil,
    ),
    // the trigger's two effects (`DamagePlayersEffect(1, OPPONENT)`, then `GainLifeEffect(1)`), as one
    Handler::effect("WhisperingSnitch.WhisperingSnitchEffect", apply)
        .effects(&["new DamagePlayersEffect(1, TargetController.OPPONENT)", "new GainLifeEffect(1)"]),
];

/// `WhisperingSnitchWatcher.getTimesSurveiled(you) == 1`: this turn's first surveil by the trigger's controller
/// (read again as it resolves: a second surveil before then stops it, where Java's check is only as it triggers)
fn first_surveil(game: &Game, item: &StackItem) -> bool {
    game.watch.surveiled.iter().filter(|s| **s == item.controller).count() == 1
}

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, cx.effects);
    (!d.stopped()).then_some(())
}
