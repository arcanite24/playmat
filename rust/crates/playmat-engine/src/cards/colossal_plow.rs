//! Colossal Plow: "Whenever Colossal Plow attacks, add {W}{W}{W} and you gain 3 life. Until end of turn, you don't
//! lose this mana as steps and phases end. Crew 6"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ColossalPlow.ColossalPlowEffect", apply)];

/// The mana (kept until end of turn), then the life
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mut colored = [0; 5];
    colored[crate::card::Color::W.index()] = 3;
    cx.run(game, d, &[EffectDef::BasicManaEffect { colored, colorless: 0, until_end_of_turn: true }]);
    gain_life(game, cx.controller(), 3);
    Some(())
}
