//! Flamebraider: "{T}: Add two mana in any combination of colors. Spend this mana only to cast Elemental spells or
//! activate abilities of Elemental sources."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::spend("Flamebraider.FlamebraiderManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    game.has_subtype(cx.on, "Elemental")
}
