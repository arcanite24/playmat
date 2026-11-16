//! Necromancer's Stockpile: "{1}{B}, Discard a creature card: Draw a card. If the discarded card was a Zombie card,
//! create a tapped 2/2 black Zombie creature token."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("NecromancersStockpile.NecromancersStockpileCondition", discarded_zombie)];

/// `DiscardTargetCost.getCards` of the resolving ability: one of them is a Zombie card
fn discarded_zombie(game: &Game, _: Seat, _: ObjId) -> bool {
    game.resolving_item.as_ref().is_some_and(|i| i.cost_discarded.iter().any(|c| game.has_subtype(*c, "Zombie")))
}
