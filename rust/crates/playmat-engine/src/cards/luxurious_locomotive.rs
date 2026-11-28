//! Luxurious Locomotive: "Whenever Luxurious Locomotive attacks, create a Treasure token for each creature that
//! crewed it this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("LuxuriousLocomotive.LuxuriousLocomotiveValue", crewed)];

/// `CrewedVehicleWatcher.getCrewCount`: the different creatures that crewed it (as it is now) this turn
fn crewed(game: &Game, _you: Seat, source: ObjId, _x: i32) -> i32 {
    let zcc = game.obj(source).zcc;
    let mut seen: Vec<(ObjId, u32)> = Vec::new();
    for (v, vz, c, cz) in &game.watch.crewed {
        if *v == source && *vz == zcc && !seen.contains(&(*c, *cz)) {
            seen.push((*c, *cz));
        }
    }
    seen.len() as i32
}
