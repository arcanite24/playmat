//! Prehistoric Pet: "This creature can't be blocked by creatures with greater power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("PrehistoricPet.PrehistoricPetPredicate", greater_power)];

/// Its power is greater than the source permanent's (`getSourcePermanentIfItStillExists`); false with none
fn greater_power(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    if game.obj(src).zone != Zone::Battlefield {
        return false;
    }
    game.power_toughness(src).0 < game.power_toughness(id).0
}
