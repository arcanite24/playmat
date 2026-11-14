//! Chandra's Revolution: "Chandra's Revolution deals 4 damage to target creature. Tap target land. That land doesn't
//! untap during its controller's next untap step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChandrasRevolution.ChandrasRevolutionEffect", apply)
    .effects(&["new DontUntapInControllersNextUntapStepTargetEffect(\"that land\")"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if let Some(o) = cx.obj(0) {
        deal_damage(game, cx.source_id(), Target::Obj(o), 4, false);
    }
    if let Some(land) = cx.obj(1).filter(|l| game.obj(*l).zone == Zone::Battlefield) {
        tap(game, land);
        cx.run_on(game, d, cx.effects, vec![Target::Obj(land)]);
    }
    Some(())
}
