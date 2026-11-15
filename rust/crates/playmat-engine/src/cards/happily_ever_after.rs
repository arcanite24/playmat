//! Happily Ever After: "When Happily Ever After enters, each player gains 5 life and draws a card. At the beginning
//! of your upkeep, if there are five colors among permanents you control, there are six or more card types among
//! permanents you control and/or cards in your graveyard, and your life total is greater than or equal to your
//! starting life total, you win the game."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[Handler::condition("HappilyEverAfter.HappilyEverAfterCondition", ever_after)];

const COLORS: [Color; 5] = [Color::W, Color::U, Color::B, Color::R, Color::G];

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

/// life at least the starting life, all five colors among your permanents, and six card types among them and the
/// cards in your graveyard
fn ever_after(game: &Game, you: Seat, _source: ObjId) -> bool {
    if game.player(you).life < game.starting_life {
        return false;
    }
    let mine: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| game.obj(*o).controller == you).collect();
    if !COLORS.iter().all(|c| mine.iter().any(|o| game.colors(*o).contains(c))) {
        return false;
    }
    let graveyard = &game.player(you).graveyard;
    let types = TYPES
        .iter()
        .filter(|t| mine.iter().any(|o| game.has_type(*o, **t)) || graveyard.iter().any(|c| game.obj(*c).is(**t)))
        .count();
    types >= 6
}
