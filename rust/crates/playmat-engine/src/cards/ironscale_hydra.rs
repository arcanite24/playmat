//! Ironscale Hydra: "If a creature would deal combat damage to Ironscale Hydra, prevent that damage and put a +1/+1
//! counter on Ironscale Hydra."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "IronscaleHydra.IronscaleHydraEffect",
    &[EventType::DamagePermanent],
    applies,
    replace,
)];

/// `PreventionEffectImpl(…, Integer.MAX_VALUE, true, false)`: combat damage that can be prevented, to this, from a
/// creature on the battlefield
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(src) = ev.source else { return false };
    cx.preventable(ev, true)
        && ev.target == Some(cx.source)
        && game.obj(src).zone == Zone::Battlefield
        && game.has_type(src, CardType::Creature)
}

/// the +1/+1 counter first (`AddCountersSourceEffect`), then all of the damage prevented
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if game.obj(cx.source).zone == Zone::Battlefield {
        add_counters_by(game, cx.source, "+1/+1", 1, Some(cx.controller));
    }
    cx.prevent(ev, i32::MAX);
    false
}
