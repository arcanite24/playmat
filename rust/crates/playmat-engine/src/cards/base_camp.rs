//! Base Camp: "{T}: Add one mana of any color. Spend this mana only to cast a Cleric, Rogue, Warrior, or Wizard
//! spell or to activate an ability of a Cleric, Rogue, Warrior, or Wizard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::spend("BaseCamp.BaseCampManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    ["Cleric", "Rogue", "Warrior", "Wizard"].iter().any(|t| game.has_subtype(cx.on, t))
}
