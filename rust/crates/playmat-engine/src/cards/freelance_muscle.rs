//! `GreatestAmongPermanentsValue.POWER_OR_TOUGHNESS_OTHER_CONTROLLED_CREATURES`: "the greatest power and/or
//! toughness among other creatures you control" (Freelance Muscle)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value(
    "GreatestAmongPermanentsValue.POWER_OR_TOUGHNESS_OTHER_CONTROLLED_CREATURES",
    greatest_power_or_toughness,
)];

/// the greatest of each other creature you control's power and toughness, 0 with none (`Quality.PowerOrToughness`)
fn greatest_power_or_toughness(game: &Game, you: Seat, source: ObjId, _: i32) -> i32 {
    game.battlefield
        .iter()
        .filter(|o| **o != source && game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .map(|o| {
            let (p, t) = game.power_toughness(*o);
            p.max(t)
        })
        .max()
        .unwrap_or(0)
}
