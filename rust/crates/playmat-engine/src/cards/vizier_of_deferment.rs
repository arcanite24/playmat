//! Vizier of Deferment: "Flash. When this creature enters, you may exile target creature if it attacked or blocked
//! this turn. Return that card to the battlefield under its owner's control at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VizierOfDeferment.VizierOfDefermentEffect", apply)
    .effects(&["new ExileReturnBattlefieldNextEndStepTargetEffect()"])];

/// `AttackedThisTurnWatcher.checkIfAttacked` or `BlockedThisTurnWatcher.checkIfBlocked` of the target as it is, then
/// the exile and the return at the next end step
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let me = (t, game.obj(t).zcc);
    if !game.attacked_this_turn.contains(&me) && !game.watch.blocked.contains(&me) {
        return Some(());
    }
    cx.run_on(game, d, cx.effects, vec![Target::Obj(t)]);
    if d.stopped() {
        return None;
    }
    Some(())
}
