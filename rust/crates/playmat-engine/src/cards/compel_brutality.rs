//! Compel Brutality, second mode: "Target planeswalker you control deals damage equal to its loyalty to target
//! creature or planeswalker an opponent controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CompelBrutality.CompelBrutalityEffect", apply)];

/// The planeswalker's loyalty counters as it resolves; nothing with none
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(from), Some(to)) = (cx.obj(0), cx.obj(1)) else { return Some(()) };
    if game.obj(from).zone != Zone::Battlefield || game.obj(to).zone != Zone::Battlefield {
        return Some(());
    }
    let n = game.obj(from).counter("loyalty");
    if n > 0 {
        deal_damage(game, from, Target::Obj(to), n, false);
    }
    Some(())
}
