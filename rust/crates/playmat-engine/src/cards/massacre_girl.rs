//! Massacre Girl: "When Massacre Girl enters, each other creature gets -1/-1 until end of turn. Whenever a creature
//! dies this turn, each creature other than Massacre Girl gets -1/-1 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("MassacreGirl.MassacreGirlEffect", apply).effects(&[
        "new BoostAllEffect(-1, -1, Duration.EndOfTurn, StaticFilters.FILTER_PERMANENT_CREATURES, true)",
        "new CreateDelayedTriggeredAbilityEffect(new MassacreGirlDelayedTriggeredAbility())",
    ]),
    Handler::trigger(
        "MassacreGirl.MassacreGirlDelayedTriggeredAbility",
        "new DiesCreatureTriggeredAbility(new BoostAllEffect(-1, -1, Duration.EndOfTurn, StaticFilters.FILTER_PERMANENT_CREATURES, true), false, StaticFilters.FILTER_PERMANENT_CREATURE)",
        any_creature,
    ),
];

/// `game.addEffect(new BoostAllEffect(…))`, then `game.addDelayedTriggeredAbility(…)`: both as this ability's
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, cx.effects);
    Some(())
}

/// a creature dying (`isDiesEvent`, a creature): the base trigger is all of it
fn any_creature(_: &Game, _: &StackItem) -> bool {
    true
}
