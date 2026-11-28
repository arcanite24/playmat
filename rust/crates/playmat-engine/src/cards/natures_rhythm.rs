//! Nature's Rhythm: "Search your library for a creature card with mana value X or less, put it onto the
//! battlefield, then shuffle. Harmonize {X}{G}{G}{G}{G}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("NaturesRhythm.NaturesRhythmPredicate", at_most_x)];

/// Its mana value is at most the spell's X (`GetXValue`): the resolving spell's, or the one on the stack
fn at_most_x(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let x = game
        .resolving_item
        .as_ref()
        .filter(|i| i.card == src && i.ability.is_none())
        .or_else(|| game.stack.iter().rev().find(|i| i.card == src && i.ability.is_none()))
        .map_or(0, |i| i.x);
    game.obj(id).def.mana_value() as i32 <= x
}
