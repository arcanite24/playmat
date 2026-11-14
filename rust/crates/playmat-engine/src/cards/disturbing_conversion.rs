//! Disturbing Conversion: "Flash. Enchant creature. When Disturbing Conversion enters, each player mills two cards.
//! Enchanted creature gets -X/-0, where X is the number of cards in its controller's graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("DisturbingConversion.DisturbingConversionValue", minus_graveyard)];

/// minus the size of the graveyard of the controller of what the source (or, gone, its last known self) is attached to
fn minus_graveyard(game: &Game, _you: Seat, source: ObjId, _x: i32) -> i32 {
    let o = game.obj(source);
    let to = if o.zone == Zone::Battlefield { o.attached_to } else { o.lki_attached_to };
    let Some((to, _)) = to else { return 0 };
    -(game.player(game.obj(to).controller).graveyard.len() as i32)
}
