//! South Wind Avatar: "Whenever another creature you control dies, you gain life equal to its toughness."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("SouthWindAvatar.SouthWindAvatarValue", died_toughness)];

/// the creature that died (`getValue("creatureDied")`): its toughness as it last was on the battlefield
fn died_toughness(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    let Some((died, _)) = game.resolving_item.as_ref().and_then(|i| i.died) else { return 0 };
    game.obj(died).toughness_lki.unwrap_or(0)
}
