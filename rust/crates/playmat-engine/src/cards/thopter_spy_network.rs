//! Thopter Spy Network: "At the beginning of your upkeep, if you control an artifact, create a 1/1 colorless Thopter
//! artifact creature token with flying."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ThopterSpyNetwork.ThopterSpyNetworkUpkeepTriggeredAbility",
    "new BeginningOfUpkeepTriggeredAbility(new CreateTokenEffect(new ThopterColorlessToken(), 1))",
    you_control_an_artifact,
)];

/// `checkInterveningIfClause`: you control an artifact
fn you_control_an_artifact(game: &Game, t: &StackItem) -> bool {
    game.battlefield.iter().any(|o| game.obj(*o).controller == t.controller && game.has_type(*o, CardType::Artifact))
}
