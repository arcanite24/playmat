//! Corrupted Crossroads: "{T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast a spell with
//! devoid." (Java's condition asks only whether what's paid for has devoid: a spell, or an ability's source.)

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[Handler::spend("CorruptedCrossroads.BlightedCrossroadsManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    game.obj(cx.on).def.abilities.iter().any(|a| matches!(a, AbilityDef::DevoidAbility))
}
