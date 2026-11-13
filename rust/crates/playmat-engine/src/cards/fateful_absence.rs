//! Fateful Absence: "Destroy target creature or planeswalker. Its controller investigates."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("FatefulAbsence.FatefulAbsenceEffect", apply).effects(&["new InvestigateEffect()"])];

/// Its controller as it was before it's destroyed creates the Clue (`InvestigateTargetEffect`), whether or not it
/// was destroyed; nothing when the target is gone
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let p = game.obj(o).controller;
    destroy(game, o);
    cx.run_as(game, d, cx.effects, p);
    Some(())
}
