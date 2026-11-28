//! Teyo, Lightshield Expert: "When Teyo enters, target permanent you control gains hexproof until end of turn. Put a
//! +1/+1 counter on it if it's a creature. Put a loyalty counter on it if it's a planeswalker."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TeyoLightshieldExpert.TeyoLightshieldExpertEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.has_type(o, CardType::Creature) {
        put_counters(game, o, "+1/+1", 1, cx.controller());
    }
    if game.has_type(o, CardType::Planeswalker) {
        put_counters(game, o, "loyalty", 1, cx.controller());
    }
    Some(())
}
