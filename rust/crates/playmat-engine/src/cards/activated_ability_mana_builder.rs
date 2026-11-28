//! `ActivatedAbilityManaBuilder`: "Spend this mana only to activate abilities." (The Enigma Jewel, Omen Hawker)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::spend("ActivatedAbilityManaBuilder", spend)];

fn spend(_: &Game, cx: &SpendCx) -> bool {
    cx.ability
}
