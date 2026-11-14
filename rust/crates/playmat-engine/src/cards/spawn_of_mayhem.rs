//! Spawn of Mayhem: "At the beginning of your upkeep, Spawn of Mayhem deals 1 damage to each player. Then if you have
//! 10 or less life, put a +1/+1 counter on Spawn of Mayhem."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SpawnOfMayhem.SpawnOfMayhemEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if game.player(cx.controller()).life < 11 && cx.source_on_battlefield(game) {
        put_counters(game, cx.source_id(), "+1/+1", 1, cx.controller());
    }
    Some(())
}
