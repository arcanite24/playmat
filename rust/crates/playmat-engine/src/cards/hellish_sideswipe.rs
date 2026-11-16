//! Hellish Sideswipe: "As an additional cost to cast this spell, sacrifice an artifact or creature. Destroy target
//! creature or Vehicle. If the sacrificed permanent was a Vehicle, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("HellishSideswipe.SacrificedPermanentCondition", vehicle)];

/// `SacrificedPermanentCondition(Vehicle)`: one of the permanents its `SacrificeTargetCost` sacrificed was a Vehicle
/// (as it was sacrificed: its last known characteristics, else its card's)
fn vehicle(game: &Game, _: Seat, _: ObjId) -> bool {
    game.resolving_item.as_ref().is_some_and(|i| {
        i.sacrificed.iter().any(|o| {
            let obj = game.obj(*o);
            obj.lki_def.as_deref().unwrap_or(&obj.def).subtypes.iter().any(|s| s == "Vehicle")
        })
    })
}
