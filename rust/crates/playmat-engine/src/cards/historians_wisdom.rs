//! Historian's Wisdom: "Enchant artifact or creature. When Historian's Wisdom enters, if enchanted permanent is a
//! creature with the greatest power among creatures on the battlefield, draw a card. As long as enchanted permanent
//! is a creature, it gets +2/+1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("HistoriansWisdom.HistoriansWisdomCondition", greatest)];

/// The source still on the battlefield, attached to a creature whose power is at least every creature's
fn greatest(game: &Game, _you: Seat, source: ObjId) -> bool {
    if game.obj(source).zone != Zone::Battlefield {
        return false;
    }
    let Some(to) = game.attached(source) else { return false };
    if !game.has_type(to, CardType::Creature) {
        return false;
    }
    let highest = game
        .battlefield
        .iter()
        .filter(|o| game.has_type(**o, CardType::Creature))
        .map(|o| game.power_toughness(*o).0)
        .max()
        .unwrap_or(0);
    game.power_toughness(to).0 >= highest
}
