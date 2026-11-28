//! Exponential Growth: "Until end of turn, double target creature's power X times."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ExponentialGrowth.ExponentialGrowthEffect", apply)];

/// The target gets +(2^X - 1) times its power /+0 until end of turn, Java's overflow-capped arithmetic
/// (`CardUtil.overflowMultiply`, `overflowDec`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let mut multiplier: i32 = 1;
    for _ in 0..cx.item.x.max(0) {
        multiplier = multiplier.saturating_mul(2);
    }
    multiplier = multiplier.saturating_sub(1);
    let power = game.power_toughness(o).0;
    boost(game, o, multiplier.saturating_mul(power), 0, Duration::EndOfTurn);
    Some(())
}
