//! Triumphant Chomp: "Triumphant Chomp deals damage to target creature equal to 2 or the greatest power among
//! Dinosaurs you control, whichever is greater."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("TriumphantChomp.MaximumDynamicValue", two_or_greatest_dinosaur)];

/// `MaximumDynamicValue(2, GreatestAmongPermanentsValue(Power, Dinosaurs you control))`
fn two_or_greatest_dinosaur(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    let greatest = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_subtype(**o, "Dinosaur"))
        .map(|o| game.power_toughness(*o).0)
        .max()
        .unwrap_or(0);
    greatest.max(2)
}
