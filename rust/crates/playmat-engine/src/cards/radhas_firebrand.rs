//! Radha's Firebrand: "Whenever this creature attacks, target creature defending player controls with power less than
//! this creature's power can't block this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("RadhasFirebrand.RadhasFirebrandPredicate", weaker)];

/// its power is less than the source's (on the battlefield, else as it last was there)
fn weaker(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(s) = source else { return false };
    let power = if game.obj(s).zone == Zone::Battlefield {
        game.power_toughness(s).0
    } else {
        let Some(p) = game.obj(s).power_lki else { return false };
        p
    };
    power > game.power_toughness(id).0
}
