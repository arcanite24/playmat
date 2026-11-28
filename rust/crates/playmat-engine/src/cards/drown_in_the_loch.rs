//! Drown in the Loch: "Choose one — Counter target spell with mana value less than or equal to the number of cards in
//! its controller's graveyard. / Destroy target creature with mana value less than or equal to the number of cards
//! in its controller's graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("DrownInTheLoch.DrownInTheLochPredicate", at_most_graveyard)];

/// Its mana value is at most the number of cards in its controller's graveyard
fn at_most_graveyard(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let o = game.obj(id);
    let controller = if o.zone == Zone::Stack {
        game.stack.iter().rev().find(|i| i.card == id && i.ability.is_none()).map_or(o.controller, |i| i.controller)
    } else {
        o.controller
    };
    o.def.mana_value() as i32 <= game.player(controller).graveyard.len() as i32
}
