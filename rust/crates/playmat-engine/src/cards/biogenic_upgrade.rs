//! Biogenic Upgrade: "Distribute three +1/+1 counters among one, two, or three target creatures, then double the
//! number of +1/+1 counters on each of those creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BiogenicUpgrade.BiogenicUpgradeEffect", apply)];

/// Each target in turn: as many +1/+1 counters as it has, put by you
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for o in cx.objs(0) {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let n = game.obj(o).counter("+1/+1");
        if n > 0 {
            add_counters_by(game, o, "+1/+1", n, Some(you));
        }
    }
    Some(())
}
