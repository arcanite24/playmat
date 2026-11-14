//! Orb of Dragonkind: "{1}, {T}: Add two mana in any combination of colors. Spend this mana only to cast Dragon
//! spells or to activate abilities of Dragons."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::spend("OrbOfDragonkind.OrbOfDragonkindManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    game.has_subtype(cx.on, "Dragon")
}
