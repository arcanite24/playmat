//! Loxodon Lifechanter: "When Loxodon Lifechanter enters, you may have your life total become the total toughness of
//! creatures you control. {5}{W}: Loxodon Lifechanter gets +X/+X until end of turn, where X is your life total."

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("LoxodonLifechanter.SetPlayerLifeSourceEffect", set_life)
        .args(&["one_of:LoxodonLifechanterValue.instance"]),
    Handler::value("LoxodonLifechanter.LoxodonLifechanterValue2", your_life),
];

/// `LoxodonLifechanterValue`: the sum of the toughness of the creatures you control, then `setLife`
fn set_life(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let total: i32 = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .map(|o| game.power_toughness(o).1)
        .sum();
    super::set_player_life_source_effect::set_life(game, you, total);
    Some(())
}

/// `LoxodonLifechanterValue2`: your life total
fn your_life(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.player(you).life
}
