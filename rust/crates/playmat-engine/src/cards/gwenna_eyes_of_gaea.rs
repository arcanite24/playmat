//! Gwenna, Eyes of Gaea: "{T}: Add two mana in any combination of colors. Spend this mana only to cast creature
//! spells or activate abilities of a creature or creature card."

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[Handler::spend("GwennaEyesOfGaea.GwennaEyesOfGaeaManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    game.has_type(cx.on, CardType::Creature)
}
