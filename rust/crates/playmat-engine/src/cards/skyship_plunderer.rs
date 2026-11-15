//! Skyship Plunderer: "Flying. Whenever Skyship Plunderer deals combat damage to a player, for each kind of counter
//! on target permanent or player, give that permanent or player another counter of that kind."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SkyshipPlunderer.SkyshipPlundererEffect", apply)];

/// One more of each kind of counter on the target, in Java's `Counters` (a `HashMap`) order, as proliferate gives
/// them
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    match cx.target(0) {
        Some(Target::Player(p)) => {
            let kinds: Vec<String> =
                game.player(p).counters.iter().filter(|(_, k)| *k > 0).map(|(n, _)| n.clone()).collect();
            for name in java_hash_order(kinds) {
                if name == "poison" {
                    add_poison(game, p, 1);
                } else if let Some(c) = game.player_mut(p).counters.iter_mut().find(|(c, _)| *c == name) {
                    c.1 += 1;
                }
            }
        }
        Some(Target::Obj(o)) if game.obj(o).zone == Zone::Battlefield => {
            let kinds: Vec<String> =
                game.obj(o).counters.iter().filter(|(_, k)| *k > 0).map(|(n, _)| n.clone()).collect();
            for name in java_hash_order(kinds) {
                put_counters(game, o, &name, 1, you);
            }
        }
        _ => {}
    }
    Some(())
}
