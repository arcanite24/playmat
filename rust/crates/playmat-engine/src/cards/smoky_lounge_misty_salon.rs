//! Smoky Lounge // Misty Salon: "At the beginning of your first main phase, add {R}{R}. Spend this mana only to
//! cast Room spells and unlock doors."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[Handler::spend("SmokyLoungeMistySalon.SmokyLoungeManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    if cx.spell() {
        return game.has_subtype(cx.on, "Room");
    }
    matches!(cx.ability_def(game), Some(AbilityDef::RoomUnlockAbility { .. }))
}
