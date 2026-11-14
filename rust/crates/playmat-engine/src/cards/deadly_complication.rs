//! Deadly Complication: "Choose one or both — • Destroy target creature. • Put a +1/+1 counter on target suspected
//! creature you control. You may have it become no longer suspected."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DeadlyComplication.DeadlyComplicationEffect", apply)];

/// the mode's target, still there: its controller's yes makes it no longer suspected
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let message = format!("Have {} no longer be suspected?", game.obj(o).def.name);
    if choose_use(game, d, cx.controller(), &message)? {
        game.obj_mut(o).set_suspected(false);
    }
    Some(())
}
