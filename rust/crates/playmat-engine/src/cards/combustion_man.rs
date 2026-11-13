//! Combustion Man: "Whenever Combustion Man attacks, destroy target permanent unless its controller has Combustion
//! Man deal damage to them equal to his power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CombustionMan.CombustionManEffect", apply)];

/// The controller is asked "take the damage?"; no means the permanent is destroyed. The damage is the source's power
/// now (or as it last was), and nothing when that's 0 or less.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let p = game.obj(o).controller;
    let name = game.obj(cx.source_id()).def.name.clone();
    if !choose_use(game, d, p, &format!("Have {name} deal damage to you equal to its power?"))? {
        cx.run_on(game, d, &[EffectDef::DestroyTargetEffect], vec![Target::Obj(o)]);
        return Some(());
    }
    let power = game.power_toughness(cx.source_id()).0;
    if power > 0 {
        deal_damage(game, cx.source_id(), Target::Player(p), power, false);
    }
    Some(())
}
