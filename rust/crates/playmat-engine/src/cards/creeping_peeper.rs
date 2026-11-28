//! Creeping Peeper: "{T}: Add {U}. Spend this mana only to cast an enchantment spell, unlock a door, or turn a
//! permanent face up."

use super::*;
use crate::card::{AbilityDef, CardType};

pub const HANDLERS: &[Handler] = &[Handler::spend("CreepingPeeper.CreepingPeeperManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    match cx.ability_def(game) {
        Some(AbilityDef::RoomUnlockAbility { .. }) => true,
        Some(AbilityDef::TurnFaceUpAbility { .. }) => game.obj(cx.on).face_down,
        _ => cx.spell() && game.has_type(cx.on, CardType::Enchantment),
    }
}
