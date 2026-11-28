//! Freya Crescent: "{T}: Add {R}. Spend this mana only to cast an Equipment spell or activate an equip ability."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[Handler::spend("FreyaCrescent.FreyaCrescentManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    if cx.spell() {
        return game.has_subtype(cx.on, "Equipment");
    }
    matches!(cx.ability_def(game), Some(AbilityDef::EquipAbility { .. }))
}
