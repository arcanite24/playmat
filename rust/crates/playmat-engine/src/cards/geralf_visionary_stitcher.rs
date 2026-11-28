//! Geralf, Visionary Stitcher: "Zombies you control have flying. {U}, {T}, Sacrifice another nontoken creature:
//! Create an X/X blue Zombie creature token, where X is the sacrificed creature's toughness."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GeralfVisionaryStitcher.GeralfVisionaryStitcherEffect", apply)
    .tokens(&["StitcherGeralfZombieToken"])];

/// X is the toughness of what the `SacrificeTargetCost` sacrificed, as it last was; one X/X token for you
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.sacrificed_stats[1];
    let mut token = cx.tokens[0].clone();
    token.power = x;
    token.toughness = x;
    cx.create_tokens(game, d, &token, 1);
    Some(())
}
