//! The Ozolith: "Whenever a creature you control leaves the battlefield, if it had counters on it, put those counters
//! on The Ozolith. At the beginning of combat on your turn, if The Ozolith has counters on it, you may move all
//! counters from The Ozolith onto target creature."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TheOzolith.TheOzolithLeaveEffect", take),
    Handler::effect("TheOzolith.TheOzolithMoveCountersEffect", give),
];

/// `(name, n)` of `counters` in Java's `Counters` (a `HashMap`) order
fn hash_ordered(counters: &[(String, i32)]) -> Vec<(String, i32)> {
    let live: Vec<&(String, i32)> = counters.iter().filter(|(_, n)| *n > 0).collect();
    java_hash_order(live.iter().map(|(k, _)| k.clone()).collect())
        .into_iter()
        .filter_map(|k| live.iter().find(|(n, _)| *n == k).map(|(_, n)| (k, *n)))
        .collect()
}

/// each kind of counter the creature had as it left (`permanentLeftBattlefield`), onto The Ozolith if it's still
/// there
fn take(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (me, zcc) = cx.source;
    let Some(left) = cx.item.left else { return Some(()) };
    if game.obj(me).zone != Zone::Battlefield || game.obj(me).zcc != zcc {
        return Some(());
    }
    for (name, n) in hash_ordered(&game.obj(left).lki_counters) {
        add_counters_by(game, me, &name, n, Some(cx.controller()));
    }
    Some(())
}

/// each kind of counter on The Ozolith onto the target, then off The Ozolith when any were put
fn give(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let me = cx.source_id();
    let Some(to) = cx.obj(0) else { return Some(()) };
    if game.obj(me).zone != Zone::Battlefield || game.obj(to).zone != Zone::Battlefield {
        return Some(());
    }
    for (name, n) in hash_ordered(&game.obj(me).counters) {
        if add_counters_by(game, to, &name, n, Some(cx.controller())) > 0 {
            game.obj_mut(me).add_counters(&name, -n);
        }
    }
    Some(())
}
