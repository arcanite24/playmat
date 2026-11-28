//! Sangromancer: "Whenever a creature an opponent controls dies, you may gain 3 life. Whenever an opponent discards a
//! card, you may gain 3 life."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "Sangromancer.SangromancerFirstTriggeredAbility",
        "new DiesCreatureTriggeredAbility(new GainLifeEffect(3), true, StaticFilters.FILTER_OPPONENTS_PERMANENT_A_CREATURE)",
        any,
    ),
    Handler::trigger(
        "Sangromancer.SangromancerSecondTriggeredAbility",
        "new DiscardsACardOpponentTriggeredAbility(new GainLifeEffect(3), true)",
        any,
    ),
];

/// (a dies event of an opponent's creature as it last was, or an opponent's `DISCARDED_CARD`: the base triggers'
/// whole checks)
fn any(_: &Game, _: &StackItem) -> bool {
    true
}
