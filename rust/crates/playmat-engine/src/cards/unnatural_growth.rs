//! Unnatural Growth: "At the beginning of each combat, double the power and toughness of each creature you control
//! until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UnnaturalGrowth.UnnaturalGrowthEffect", apply)];

/// `BoostTargetEffect(power, toughness)` for each creature you control, with its power and toughness as the effect
/// resolves: all read before any boost applies (Java adds the effects, and they apply after the resolution)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let boosts: Vec<(ObjId, (i32, i32))> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .map(|o| (o, game.power_toughness(o)))
        .collect();
    for (o, (p, t)) in boosts {
        boost(game, o, p, t, Duration::EndOfTurn);
    }
    Some(())
}
