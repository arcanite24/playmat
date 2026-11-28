//! Interface Ace: "This creature can crew Vehicles and saddle Mounts as though its power were greater than 0.
//! Whenever this creature becomes tapped during your turn, untap it. This ability triggers only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "InterfaceAce.InterfaceAceTriggeredAbility",
    "new BecomesTappedSourceTriggeredAbility(new UntapSourceEffect().setText(\"untap it\")).setTriggersLimitEachTurn(1)",
    during_your_turn,
)];

/// `game.isActivePlayer(getControllerId())`, as it triggers
fn during_your_turn(game: &Game, item: &StackItem) -> bool {
    if game.resolving_item.as_ref().is_some_and(|r| r.card == item.card && r.ability == item.ability) {
        return true;
    }
    game.active == Some(item.controller)
}
