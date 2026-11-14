//! Hotshot Investigators: "When Hotshot Investigators enters, return up to one other target creature to its owner's
//! hand. If you controlled it, investigate."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HotshotInvestigators.HotshotInvestigatorsEffect", apply)
    .effects(&["new InvestigateEffect()"])];

/// whether you controlled it, read before it moves
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let yours = game.obj(o).controller == cx.controller();
    game.move_to(o, Zone::Hand);
    if yours {
        cx.run_on(game, d, &cx.effects[..1], Vec::new());
    }
    (!d.stopped()).then_some(())
}
