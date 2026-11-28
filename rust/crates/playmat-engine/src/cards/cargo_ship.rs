//! Cargo Ship: "{T}: Add {C}. Spend this mana only to cast an artifact spell or activate an ability of an artifact
//! source."

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[Handler::spend("CargoShip.CargoShipManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    game.has_type(cx.on, CardType::Artifact)
}
