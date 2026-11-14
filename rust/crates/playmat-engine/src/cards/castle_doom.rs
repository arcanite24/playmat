//! Castle Doom: "{T}: Add one mana of any color. Spend this mana only to cast an artifact spell."

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[Handler::spend("CastleDoom.CastleDoomManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    cx.spell() && game.has_type(cx.on, CardType::Artifact)
}
