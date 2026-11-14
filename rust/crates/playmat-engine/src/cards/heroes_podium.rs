//! Heroes' Podium: "Each legendary creature you control gets +1/+1 for each other legendary creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("HeroesPodium.HeroesPodiumValue", others)];

/// the legendary creatures you control, less one (not below 0)
fn others(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let n = game
        .battlefield
        .iter()
        .filter(|o| {
            let ob = game.obj(**o);
            ob.controller == you
                && game.has_type(**o, CardType::Creature)
                && ob.def.supertypes.iter().any(|s| s == "LEGENDARY")
        })
        .count() as i32;
    (n - 1).max(0)
}
