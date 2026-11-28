//! Smoky Lounge // Misty Salon: "At the beginning of your first main phase, add {R}{R}. Spend this mana only to
//! cast Room spells and unlock doors." Misty Salon: "When you unlock this door, create an X/X blue Spirit creature
//! token with flying, where X is the number of unlocked doors among Rooms you control."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[
    Handler::spend("SmokyLoungeMistySalon.SmokyLoungeManaBuilder", spend),
    Handler::effect("SmokyLoungeMistySalon.MistySalonEffect", misty_salon).tokens(&["SpiritBlueToken"]),
];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    if cx.spell() {
        return game.has_subtype(cx.on, "Room");
    }
    matches!(cx.ability_def(game), Some(AbilityDef::RoomUnlockAbility { .. }))
}

/// the unlocked doors of the Rooms you control, counted as it resolves; then a `SpiritBlueToken` that's X/X
fn misty_salon(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n: i32 = game
        .battlefield
        .iter()
        .filter(|c| game.obj(**c).controller == you && game.has_subtype(**c, "Room"))
        .map(|c| game.obj(*c).doors.iter().filter(|u| **u).count() as i32)
        .sum();
    let e = EffectDef::CreateXXTokenEffect { token: cx.tokens[0].clone(), amount: Amount::Fixed(n) };
    cx.run_on(game, d, &[e], Vec::new());
    (!d.stopped()).then_some(())
}
