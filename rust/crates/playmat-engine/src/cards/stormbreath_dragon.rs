//! Stormbreath Dragon: "When Stormbreath Dragon becomes monstrous, it deals damage to each opponent equal to the
//! number of cards in that player's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("StormbreathDragon.StormbreathDragonDamageEffect", apply)];

/// Each opponent in turn order, the damage their hand's size, none when it's empty
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for p in (0..game.seats()).filter(|p| *p != you) {
        let n = game.player(p).hand.len() as i32;
        if n > 0 {
            deal_damage(game, cx.source.0, Target::Player(p), n, false);
        }
    }
    Some(())
}
