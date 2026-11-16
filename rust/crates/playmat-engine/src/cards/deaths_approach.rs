//! Death's Approach: "Enchanted creature gets -X/-X, where X is the number of creature cards in its controller's
//! graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value(
    "DeathsApproach.CardsInEnchantedCreaturesControllerGraveyardCount",
    creature_cards,
)];

/// the creature cards in the graveyard of the enchanted creature's controller (the card's only filter)
fn creature_cards(game: &Game, _: Seat, source: ObjId, _: i32) -> i32 {
    let aura = game.obj(source);
    if aura.zone != Zone::Battlefield {
        return 0;
    }
    let Some((o, zcc)) = aura.attached_to else { return 0 };
    let c = game.obj(o);
    if c.zone != Zone::Battlefield || c.zcc != zcc {
        return 0;
    }
    game.player(c.controller).graveyard.iter().filter(|g| game.obj(**g).def.is(CardType::Creature)).count() as i32
}
