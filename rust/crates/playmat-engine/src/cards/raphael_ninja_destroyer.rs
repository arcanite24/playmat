//! Raphael, Ninja Destroyer: "Raphael must be blocked if able. Whenever Raphael is dealt damage, add that much {R}.
//! Until end of turn, you don't lose this mana as steps and phases end."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RaphaelNinjaDestroyer.RaphaelNinjaDestroyerEffect", apply)];

/// the damage the trigger saw (`SavedDamageValue.AMOUNT`, its X) as {R} that stays until end of turn
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.item.x;
    if n < 1 {
        return Some(());
    }
    let p = game.player_mut(cx.controller());
    p.mana_pool.colored[3] += n as u32;
    p.eot_mana.colored[3] += n as u32;
    Some(())
}
