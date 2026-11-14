//! Ray of Enfeeblement: "Target creature gets -4/-1 until end of turn. If that creature is white, it gets -4/-4
//! until end of turn instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RayOfEnfeeblement.RayOfEnfeeblementEffect", apply)];

/// Its color as it resolves
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    let white = game.colors(o).contains(&crate::card::Color::W);
    let e = EffectDef::BoostTargetEffect {
        power: crate::card::Amount::Fixed(-4),
        toughness: crate::card::Amount::Fixed(if white { -4 } else { -1 }),
        duration: crate::card::Duration::EndOfTurn,
    };
    cx.run(game, d, &[e]);
    Some(())
}
