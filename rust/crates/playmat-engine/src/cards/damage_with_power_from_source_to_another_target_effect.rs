//! `DamageWithPowerFromSourceToAnotherTargetEffect(name)`: "it deals damage equal to its power to target creature"
//! (Chainweb Aracnir): the source (or its last known information on the battlefield) deals damage equal to its power
//! to the target permanent or player.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DamageWithPowerFromSourceToAnotherTargetEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    let power = if game.obj(src).zone == Zone::Battlefield { game.power_toughness(src).0 } else { game.obj(src).power_lki? };
    let t = cx.target(0)?;
    deal_damage(game, src, t, power, false);
    Some(())
}
