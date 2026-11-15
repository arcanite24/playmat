//! Invisible Woman, Sue Storm: "Whenever you put one or more +1/+1 counters on one or more other Heroes you control,
//! you may create a 0/4 colorless Wall creature token with defender."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "InvisibleWomanSueStorm.InvisibleWomanSueStormAbility",
    "new PutCounterOnPermanentTriggeredAbility(new CreateTokenEffect(new WallColorlessToken()), CounterType.P1P1, new FilterControlledPermanent(SubType.HERO), true, true)",
    another,
)];

/// `COUNTERS_ADDED` by you on a Hero you control (the base's filter): not on this one
fn another(game: &Game, t: &StackItem) -> bool {
    let _ = game;
    !matches!(t.targets.first().and_then(|g| g.first()), Some(Target::Obj(o)) if *o == t.card)
}
