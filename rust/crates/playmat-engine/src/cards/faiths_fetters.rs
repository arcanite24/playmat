//! Faith's Fetters: "Enchanted permanent can't attack or block, and its activated abilities can't be activated
//! unless they're mana abilities."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("FaithsFetters.FaithsFettersEffect", Rule::CantActivate(enchanted))];

/// `ACTIVATE_ABILITY` of a non-mana activated ability whose source this Aura is attached to
fn enchanted(game: &Game, cx: &StaticCx, _: Seat, id: ObjId, _: usize) -> bool {
    game.attached(cx.source) == Some(id)
}
