//! Display of Dominance: "Choose one — • Destroy target blue or black noncreature permanent. • Permanents you control
//! can't be the targets of blue or black spells your opponents control this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("DisplayOfDominance.DisplayOfDominanceEffect", Rule::CantBeTargeted(blue_or_black_spell))];

/// `TARGET` by an opponent's spell (on the stack) that's blue or black, of a permanent you control
fn blue_or_black_spell(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, src: ObjId) -> bool {
    let spell = game.obj(src).zone == Zone::Stack && game.stack.iter().any(|i| i.card == src && i.ability.is_none());
    spell
        && cx.opponent(seat)
        && game.colors(src).iter().any(|c| matches!(c, Color::U | Color::B))
        && game.obj(id).controller == cx.controller
}
