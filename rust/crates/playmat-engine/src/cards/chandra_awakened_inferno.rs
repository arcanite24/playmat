//! Chandra, Awakened Inferno: "+2: Each opponent gets an emblem with "At the beginning of your upkeep, this emblem
//! deals 1 damage to you.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChandraAwakenedInferno.ChandraAwakenedInfernoEffect", apply)
    .effects(&["new GetEmblemEffect(new ChandraAwakenedInfernoEmblem())"])];

/// `game.addEmblem` for each opponent in turn order: the emblem is theirs
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        cx.run_as(game, d, &cx.effects[..1], p);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
