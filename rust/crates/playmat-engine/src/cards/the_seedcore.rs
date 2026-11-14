//! The Seedcore: "{T}: Add one mana of any color. Spend this mana only to cast Phyrexian creature spells."

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[Handler::spend("TheSeedcore.TheSeedcoreManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    cx.spell() && game.has_type(cx.on, CardType::Creature) && game.has_subtype(cx.on, "Phyrexian")
}
