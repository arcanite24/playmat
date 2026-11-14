//! Anticausal Vestige: "When this creature leaves the battlefield, draw a card, then you may put a permanent card with
//! mana value less than or equal to the number of lands you control from your hand onto the battlefield tapped."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("AnticausalVestige.AnticausalVestigePredicate", at_most_your_lands)];

/// its mana value is at most the number of lands you (the ability's controller) control (`LandsYouControlCount`)
fn at_most_your_lands(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let lands = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Land))
        .count() as u32;
    game.obj(id).def.mana_value() <= lands
}
