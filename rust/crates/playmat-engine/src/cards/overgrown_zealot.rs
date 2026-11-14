//! Overgrown Zealot: "{T}: Add two mana of any one color. Spend this mana only to turn permanents face up."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[Handler::spend("OvergrownZealot.OvergrownZealotManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    matches!(cx.ability_def(game), Some(AbilityDef::TurnFaceUpAbility { .. })) && game.obj(cx.on).face_down
}
