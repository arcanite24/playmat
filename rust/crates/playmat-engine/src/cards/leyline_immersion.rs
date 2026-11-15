//! Leyline Immersion: "Enchant legendary creature. Enchanted creature has ward {2} and "{T}: Add five mana in any
//! combination of colors. Spend this mana only to cast spells.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::spend("LeylineImmersion.LeylineImmersionManaBuilder", spend)];

/// `source instanceof SpellAbility`
fn spend(_: &Game, cx: &SpendCx) -> bool {
    cx.spell()
}
