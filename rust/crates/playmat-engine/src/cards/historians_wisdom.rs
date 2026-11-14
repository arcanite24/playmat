//! Historian's Wisdom: "Enchant artifact or creature. When Historian's Wisdom enters, if enchanted permanent is a
//! creature with the greatest power among creatures on the battlefield, draw a card. As long as enchanted permanent
//! is a creature, it gets +2/+1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("HistoriansWisdom.HistoriansWisdomCondition", greatest)];

/// While its own spell resolves, the Aura's enters trigger is checked before the engine attaches it (Java's Aura
/// enters already attached): the spell's target, still on the battlefield, is what it enters attached to
fn entering_attached_to(game: &Game, source: ObjId) -> Option<ObjId> {
    let item = game.resolving_item.as_ref().filter(|i| i.card == source && i.ability.is_none())?;
    let Some(Target::Obj(to)) = item.targets.first()?.first().copied() else { return None };
    (game.obj(to).zone == Zone::Battlefield).then_some(to)
}

/// The source still on the battlefield, attached to a creature whose power is at least every creature's
fn greatest(game: &Game, _you: Seat, source: ObjId) -> bool {
    if game.obj(source).zone != Zone::Battlefield {
        return false;
    }
    let Some(to) = game.attached(source).or_else(|| entering_attached_to(game, source)) else { return false };
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
