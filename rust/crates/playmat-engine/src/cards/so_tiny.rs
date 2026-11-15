//! So Tiny: "Enchanted creature gets -2/-0. It gets -6/-0 instead as long as its controller has seven or more cards
//! in their graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("SoTiny.SoTinyValue", minus_power)];

/// -6 when the enchanted creature's controller has seven or more cards in their graveyard, else -2
fn minus_power(game: &Game, _: Seat, source: ObjId, _: i32) -> i32 {
    if game.obj(source).zone != Zone::Battlefield {
        return -2;
    }
    let Some(to) = game.attached(source) else { return -2 };
    if game.player(game.obj(to).controller).graveyard.len() >= 7 {
        -6
    } else {
        -2
    }
}
