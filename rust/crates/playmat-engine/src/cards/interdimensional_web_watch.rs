//! Interdimensional Web Watch: "{T}: Add two mana in any combination of colors. Spend this mana only to cast spells
//! from exile."

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::spend("InterdimensionalWebWatch.InterdimensionalWebWatchManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    let o = game.obj(cx.on);
    cx.spell() && if o.zone == Zone::Stack { o.cast_from == Some(Zone::Exile) } else { o.zone == Zone::Exile }
}
