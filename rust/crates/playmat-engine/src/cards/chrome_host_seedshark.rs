//! Chrome Host Seedshark: "Whenever you cast a noncreature spell, incubate X, where X is that spell's mana value."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChromeHostSeedshark.ChromeHostSeedsharkEffect", apply)
    .effects(&["new IncubateEffect(0)"])];

/// `IncubateEffect.doIncubate(spell.getManaValue())`: a cast trigger's X is the spell's mana value
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mut incubate = cx.effects[0].clone();
    if let EffectDef::IncubateEffect { amount, .. } = &mut incubate {
        *amount = cx.item.x;
    }
    cx.run(game, d, &[incubate]);
    Some(())
}
