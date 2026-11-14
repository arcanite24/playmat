//! Voldaren Estate: "{T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast a Vampire spell."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::spend("VoldarenEstate.VoldarenEstateManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    cx.spell() && game.has_subtype(cx.on, "Vampire")
}
