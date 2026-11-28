//! Earthshaker Khenra: "When Earthshaker Khenra enters, target creature with power less than or equal to Earthshaker
//! Khenra's power can't block this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("EarthshakerKhenra.EarthshakerKhenraPredicate", at_most_khenra_power)];

/// Its power is at most the source's (`getSourcePermanentOrLKI`); false with no source
fn at_most_khenra_power(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let power = if game.obj(src).zone == Zone::Battlefield {
        game.power_toughness(src).0
    } else {
        match game.obj(src).power_lki {
            Some(p) => p,
            None => return false,
        }
    };
    game.power_toughness(id).0 <= power
}
