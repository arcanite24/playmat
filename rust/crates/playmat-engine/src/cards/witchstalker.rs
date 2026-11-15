//! Witchstalker: "Hexproof. Whenever an opponent casts a blue or black spell during your turn, put a +1/+1 counter
//! on Witchstalker."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "Witchstalker.WitchstalkerTriggeredAbility",
    "new SpellCastOpponentTriggeredAbility(new AddCountersSourceEffect(CounterType.P1P1.createInstance()), filter, false)",
    during_your_turn,
)];

/// `game.isActivePlayer(getControllerId())` (the spell's colors are the base trigger's filter)
fn during_your_turn(game: &Game, item: &StackItem) -> bool {
    game.active == Some(item.controller)
}
