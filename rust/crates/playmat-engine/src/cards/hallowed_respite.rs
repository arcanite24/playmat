//! Hallowed Respite: "Exile target nonlegendary creature, then return it to the battlefield under its owner's
//! control. If it entered under your control, put a +1/+1 counter on it. Otherwise, tap it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HallowedRespite.HallowedRespiteEffect", apply)];

/// The returned permanent (the same card, `source.getFirstTarget()`): a +1/+1 counter if you control it, else tapped
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(Target::Obj(o)) = cx.item.targets.first().and_then(|g| g.first()).copied() else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let you = cx.controller();
    if game.obj(o).controller == you {
        add_counters_by(game, o, "+1/+1", 1, Some(you));
    } else {
        tap(game, o);
    }
    Some(())
}
