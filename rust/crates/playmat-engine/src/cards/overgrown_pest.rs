//! Overgrown Pest: "When this creature enters, look at the top five cards of your library. You may reveal a land or
//! double-faced card from among them and put it into your hand. Put the rest on the bottom of your library in a random
//! order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("OvergrownPest.DoubleFacedCardPredicate", double_faced)];

/// `DoubleFacedCardPredicate`: a modal double-faced card or a card with a second face
fn double_faced(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let o = game.obj(id);
    let def = o.front.as_ref().unwrap_or(&o.def);
    def.back.is_some() || def.transform_back.is_some()
}
