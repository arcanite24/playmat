//! Splitting the Powerstone: "As an additional cost to cast this spell, sacrifice an artifact. Create two tapped
//! Powerstone tokens. If the sacrificed artifact was legendary, draw a card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("SplittingThePowerstone.SplittingThePowerstoneCondition", sacrificed_legendary)];

/// `SacrificeTargetCost.getPermanents`: one of the sacrificed permanents was legendary
fn sacrificed_legendary(game: &Game, _you: Seat, source: ObjId) -> bool {
    game.resolving_item.as_ref().is_some_and(|i| {
        i.card == source && i.sacrificed.iter().any(|s| game.obj(*s).def.supertypes.iter().any(|t| t == "LEGENDARY"))
    })
}
