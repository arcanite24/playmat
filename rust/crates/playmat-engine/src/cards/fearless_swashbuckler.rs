//! Fearless Swashbuckler: "Haste. Vehicles you control have haste. Whenever you attack, if a Pirate and a Vehicle
//! attacked this combat, draw three cards, then discard two cards."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("FearlessSwashbuckler.FearlessSwashbucklerCondition", pirate_and_vehicle)];

/// `FearlessSwashbucklerWatcher`: a Pirate and a Vehicle among this combat's declared attackers (any player's)
fn pirate_and_vehicle(game: &Game, _: Seat, _: ObjId) -> bool {
    let attacked = || game.attack_taps.iter().map(|(a, _)| *a);
    attacked().any(|a| game.has_subtype(a, "Pirate")) && attacked().any(|a| game.has_subtype(a, "Vehicle"))
}
