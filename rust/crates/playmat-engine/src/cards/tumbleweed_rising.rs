//! Tumbleweed Rising: "Create an X/X green Elemental creature token, where X is the greatest power among creatures
//! you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TumbleweedRising.TumbleweedRisingEffect", apply)
    .effects(&["new CreateTokenEffect(new ElementalXXGreenToken())"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = super::miming_slime::greatest_power(game, cx.controller());
    let mut e = cx.effects[0].clone();
    if let EffectDef::CreateTokenEffect { token, .. } = &mut e {
        token.power = x;
        token.toughness = x;
    }
    cx.run(game, d, &[e]);
    Some(())
}
