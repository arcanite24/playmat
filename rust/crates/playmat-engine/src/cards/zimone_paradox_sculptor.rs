//! Zimone, Paradox Sculptor: "{G}{U}, {T}: Double the number of each kind of counter on up to two target creatures
//! and/or artifacts you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ZimoneParadoxSculptor.ZimoneParadoxSculptorEffect", apply)];

/// Each target in order; on each, every kind of counter it has gets as many more, kinds in the order of Java's
/// counters `HashMap`
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for o in cx.objs(0) {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let kinds: Vec<(String, i32)> = game.obj(o).counters.iter().filter(|(_, n)| *n > 0).cloned().collect();
        for name in java_hash_order(kinds.iter().map(|(n, _)| n.clone()).collect()) {
            let n = kinds.iter().find(|(k, _)| *k == name).map_or(0, |(_, n)| *n);
            put_counters(game, o, &name, n, cx.controller());
        }
    }
    Some(())
}
