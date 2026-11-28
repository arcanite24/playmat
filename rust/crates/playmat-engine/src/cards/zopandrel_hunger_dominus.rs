//! Zopandrel, Hunger Dominus: "At the beginning of each combat, double the power and toughness of each creature you
//! control until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ZopandrelHungerDominus.ZopandrelHungerDominusEffect", apply)];

/// Each creature you control, in battlefield order, gets +X/+Y until end of turn where X and Y are its power and
/// toughness as the effect begins for it (`BoostTargetEffect(power, toughness)` on a `FixedTarget`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mine: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    // Java reads every power and toughness before any boost applies (effects apply after `apply` returns)
    let pts: Vec<(ObjId, (i32, i32))> = mine.into_iter().map(|o| (o, game.power_toughness(o))).collect();
    for (o, (p, t)) in pts {
        boost(game, o, p, t, Duration::EndOfTurn);
    }
    Some(())
}
