//! Host of the Hereafter: "This creature enters with two +1/+1 counters on it. Whenever this creature or another
//! creature you control dies, if it had counters on it, put its counters on up to one target creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HostOfTheHereafter.HostOfTheHereafterDiesEffect", apply)];

/// The target still on the battlefield gets each kind of counter the creature that died had as it died
/// (`getValue("creatureDied")`), as many, in Java's `Counters` (a `HashMap`) order
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(to) = cx.obj(0) else { return Some(()) };
    let Some((dead, _)) = cx.item.died else { return Some(()) };
    if game.obj(to).zone != Zone::Battlefield {
        return Some(());
    }
    let counters: Vec<(String, i32)> = game.obj(dead).lki_counters.iter().filter(|(_, n)| *n > 0).cloned().collect();
    let kinds = java_hash_order(counters.iter().map(|(k, _)| k.clone()).collect());
    let you = cx.controller();
    for k in kinds {
        let n = counters.iter().find(|(c, _)| *c == k).map_or(0, |(_, n)| *n);
        if n > 0 && game.obj(to).zone == Zone::Battlefield {
            put_counters(game, to, &k, n, you);
        }
    }
    Some(())
}
