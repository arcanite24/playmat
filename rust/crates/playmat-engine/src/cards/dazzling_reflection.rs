//! Dazzling Reflection: "You gain life equal to target creature's power. The next time that creature would deal
//! damage this turn, prevent that damage."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("DazzlingReflection.DazzlingReflectionEffect", apply)
        .effects(&["new DazzlingReflectionPreventEffect()"]),
    Handler::replacement(
        "DazzlingReflection.DazzlingReflectionPreventEffect",
        &[EventType::DamagePlayer, EventType::DamagePermanent],
        applies,
        replace,
    ),
];

/// You gain life equal to the target's power, then the prevention starts this turn, pointed at that creature
/// (`FixedTarget(targetCreature, game)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let creature = cx.obj(0)?;
    let power = game.power_toughness(creature).0;
    gain_life(game, cx.controller(), power);
    cx.run_on(game, d, cx.effects, vec![Target::Obj(creature)]);
    if d.stopped() {
        return None;
    }
    Some(())
}

/// `PreventionEffectImpl.applies` (damage that can be prevented, combat or not) from that creature, still that
/// object, until it has prevented once
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.preventable(ev, false)
        && cx.left(game, 1) > 0
        && cx.target_obj(game, 0).is_some_and(|o| ev.source == Some(o))
}

/// All of that damage prevented, then `discard()`
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    cx.set_left(game, 0);
    false
}
