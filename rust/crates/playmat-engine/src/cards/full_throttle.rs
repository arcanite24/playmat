//! Full Throttle: "After this main phase, there are two additional combat phases. At the beginning of each combat
//! this turn, untap all creatures that attacked this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("FullThrottle.FullThrottleCondition", in_main_phase),
    Handler::trigger(
        "FullThrottle.FullThrottleTriggeredAbility",
        "new BeginningOfCombatTriggeredAbility(TargetController.ANY, new UntapAllEffect(filter), false)",
        any_combat,
    ),
];

/// `game.getTurnPhaseType().isMain()`
fn in_main_phase(game: &Game, _: Seat, _: ObjId) -> bool {
    matches!(game.step, Some(Step::PrecombatMain | Step::PostcombatMain))
}

/// `COMBAT_PHASE_PRE`: each combat (the base trigger is all of it)
fn any_combat(_: &Game, _: &StackItem) -> bool {
    true
}
