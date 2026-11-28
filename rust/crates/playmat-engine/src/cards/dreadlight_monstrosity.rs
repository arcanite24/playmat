//! Dreadlight Monstrosity: "{3}{U}{U}: Dreadlight Monstrosity can't be blocked this turn. Activate only if you own a
//! card in exile."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("DreadlightMonstrosity.DreadlightMonstrosityCondition", own_card_in_exile)];

/// A card in exile you own
fn own_card_in_exile(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.exile.iter().any(|c| game.obj(*c).owner == you)
}
