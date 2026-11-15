//! Overzealous Muscle: "Whenever you commit a crime during your turn, Overzealous Muscle gains indestructible until
//! end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "OverzealousMuscle.OverzealousMuscleTriggeredAbility",
    "new CommittedCrimeTriggeredAbility(new GainAbilitySourceEffect(IndestructibleAbility.getInstance(), Duration.EndOfTurn))",
    your_turn,
)];

/// `game.isActivePlayer(getControllerId())`
fn your_turn(game: &Game, item: &StackItem) -> bool {
    game.active == Some(item.controller)
}
