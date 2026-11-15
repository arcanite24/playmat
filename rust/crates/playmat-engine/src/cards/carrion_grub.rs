//! Carrion Grub: "Carrion Grub gets +X/+0, where X is the greatest power among creature cards in your graveyard.
//! When Carrion Grub enters, mill four cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("CarrionGrub.CarrionGrubValue", greatest_power)];

/// the greatest power among creature cards in your graveyard (0 without any)
fn greatest_power(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.player(you)
        .graveyard
        .iter()
        .filter(|c| game.obj(**c).is(CardType::Creature))
        .map(|c| game.obj(*c).def.power)
        .max()
        .unwrap_or(0)
}
