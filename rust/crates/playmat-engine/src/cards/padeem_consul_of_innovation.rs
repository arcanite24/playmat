//! Padeem, Consul of Innovation: "At the beginning of your upkeep, if you control the artifact with the highest mana
//! value or tied for the highest mana value, draw a card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("PadeemConsulOfInnovation.PadeemConsulOfInnovationPredicate", greatest_mana_value)];

/// Its mana value is at least every artifact's on the battlefield (`getActivePermanents` in its controller's range:
/// all of them in a duel)
fn greatest_mana_value(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let most = game
        .battlefield
        .iter()
        .filter(|o| game.has_type(**o, CardType::Artifact))
        .map(|o| game.obj(*o).def.mana_value() as i32)
        .max()
        .unwrap_or(-1);
    game.obj(id).def.mana_value() as i32 >= most
}
