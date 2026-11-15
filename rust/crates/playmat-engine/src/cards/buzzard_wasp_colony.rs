//! Buzzard-Wasp Colony: "Flying. When this creature enters, you may sacrifice an artifact or creature. If you do,
//! draw a card. Whenever another creature you control dies, if it had counters on it, put its counters on this
//! creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BuzzardWaspColony.BuzzardWaspColonyEffect", apply)];

/// This creature, while it's still the permanent it was, gets each kind of counter the creature that died had as
/// it died (`getValue("creatureDied")`), as many, in Java's `Counters` (a `HashMap`) order
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let to = cx.source_id();
    let Some((dead, _)) = cx.item.died else { return Some(()) };
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
