//! Shields Up!: "Target artifact or creature you control gains hexproof and indestructible until end of turn. If
//! it's a creature, put a +1/+1 counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShieldsUp.ShieldsUpEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone == Zone::Battlefield && game.has_type(o, CardType::Creature) {
        put_counters(game, o, "+1/+1", 1, cx.controller());
    }
    Some(())
}
