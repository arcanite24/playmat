//! Loot, the Key to Everything: "At the beginning of your upkeep, exile the top X cards of your library, where X is
//! the number of card types among other nonland permanents you control. You may play those cards this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("LootTheKeyToEverything.LootTheKeyToEverythingValue", types)];

const TYPES: [CardType; 9] = [
    CardType::Artifact,
    CardType::Battle,
    CardType::Creature,
    CardType::Enchantment,
    CardType::Instant,
    CardType::Kindred,
    CardType::Land,
    CardType::Planeswalker,
    CardType::Sorcery,
];

fn types(game: &Game, you: Seat, source: ObjId, _x: i32) -> i32 {
    let mine: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|p| *p != source && game.obj(*p).controller == you && !game.has_type(*p, CardType::Land))
        .collect();
    TYPES.iter().filter(|t| mine.iter().any(|p| game.has_type(*p, **t))).count() as i32
}
