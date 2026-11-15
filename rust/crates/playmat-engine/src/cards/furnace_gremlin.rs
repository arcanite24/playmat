//! Furnace Gremlin: "Whenever Furnace Gremlin attacks, incubate X, where X is its power."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("FurnaceGremlin.FurnaceGremlinEffect", apply).effects(&["new IncubateEffect(0)"])];

/// the source's power as it last was on the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = game.obj(cx.source_id());
    let power = if o.zone == Zone::Battlefield {
        game.power_toughness(cx.source_id()).0
    } else {
        o.power_lki.unwrap_or(o.def.power)
    };
    let mut e = cx.effects[0].clone();
    if let EffectDef::IncubateEffect { amount, .. } = &mut e {
        *amount = power;
    }
    cx.run(game, d, &[e]);
    Some(())
}
