//! Kruphix, God of Horizons: "Indestructible. As long as your devotion to green and blue is less than seven, Kruphix
//! isn't a creature. You have no maximum hand size. If unused mana would empty from your mana pool, that mana
//! becomes colorless instead."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("KruphixGodOfHorizons.KruphixGodOfHorizonsEffect", Rule::ManaBecomes(colorless))];

/// its controller's unspent mana becomes colorless (`setManaBecomesColorless`)
fn colorless(_: &Game, cx: &StaticCx, seat: Seat) -> Option<usize> {
    (seat == cx.controller).then_some(5)
}
