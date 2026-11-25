//! Ral Zarek, -7: "Flip five coins. Take an extra turn after this one for each coin that comes up heads."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RalZarek.RalZarekExtraTurnsEffect", apply)];

/// `player.flipCoins(source, game, 5, true)`: five winnable flips one after another, each won flip an extra turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for _ in 0..5 {
        let won = vec![EffectDef::AddExtraTurnControllerEffect];
        cx.run(game, d, &[EffectDef::FlipCoinEffect { won, lost: Vec::new() }]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
