//! Ambitious Augmenter: "Increment. When this creature dies, if it had one or more counters on it, create a 0/0
//! green and blue Fractal creature token, then put this creature's counters on that token."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("AmbitiousAugmenter.AmbitiousAugmenterCondition", had_counters),
    Handler::effect("AmbitiousAugmenter.AmbitiousAugmenterEffect", apply)
        .effects(&["new CreateTokenEffect(new FractalToken())"]),
];

/// the creature had one or more counters on it as it left the battlefield (`permanentLeftBattlefield`)
/// (read as the trigger is collected, with the creature still on the battlefield, and as it resolves, from its last
/// known counters, as `Value::SourceCountersAny`)
fn had_counters(game: &Game, _you: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    let counters = if o.zone == Zone::Battlefield { &o.counters } else { &o.lki_counters };
    counters.iter().map(|(_, n)| *n).sum::<i32>() >= 1
}

/// The Fractal (`getLastAddedTokenIds`: the tokens made now), then on each still on the battlefield the counters this
/// creature had as it died, each kind in Java's `Counters` (a `HashMap`) order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    let counters: Vec<(String, i32)> = game.obj(me).lki_counters.iter().filter(|(_, n)| *n > 0).cloned().collect();
    let first = game.objs.len();
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = (first..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    let kinds = java_hash_order(counters.iter().map(|(k, _)| k.clone()).collect());
    for t in made {
        for k in &kinds {
            let n = counters.iter().find(|(c, _)| c == k).map_or(0, |(_, n)| *n);
            if n > 0 && game.obj(t).zone == Zone::Battlefield {
                put_counters(game, t, k, n, you);
            }
        }
    }
    Some(())
}
