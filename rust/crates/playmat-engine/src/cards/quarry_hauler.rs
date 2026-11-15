//! Quarry Hauler: "When Quarry Hauler enters, for each kind of counter on target permanent, put another counter of
//! that kind on it or remove one from it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("QuarryHauler.QuarryHaulerEffect", apply)];

/// The kinds on it as it starts (`Counters`' hash order), each asked in turn: yes ("Add") puts one more, no
/// ("Remove") takes one off
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let you = cx.controller();
    let kinds = java_hash_order(game.obj(o).counters.iter().filter(|(_, n)| *n > 0).map(|(c, _)| c.clone()).collect());
    for kind in kinds {
        if choose_use(game, d, you, &format!("Add or remove a {kind} counter?"))? {
            add_counters_by(game, o, &kind, 1, Some(you));
        } else if game.obj(o).counter(&kind) > 0 {
            let ob = game.obj_mut(o);
            ob.add_counters(&kind, -1);
            ob.counters.retain(|(_, k)| *k > 0);
        }
    }
    Some(())
}
